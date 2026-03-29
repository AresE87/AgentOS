// R8 — Mesh Network page
import { useState, useEffect } from 'react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import EmptyState from '../../components/EmptyState';
import { useAgent } from '../../hooks/useAgent';
import { Network, Monitor, Wifi, WifiOff } from 'lucide-react';
import type { MeshNode } from '../../types/ipc';

export default function Mesh() {
  const { getMeshNodes } = useAgent();
  const [nodes, setNodes] = useState<MeshNode[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetch = async () => {
      try {
        const result = await getMeshNodes();
        setNodes(result.nodes || []);
      } catch { /* ignore */ }
      setLoading(false);
    };
    fetch();
    const interval = setInterval(fetch, 10000);
    return () => clearInterval(interval);
  }, []);

  const selfNode = nodes.find(n => n.address.includes('127.0.0.1'));
  const remoteNodes = nodes.filter(n => !n.address.includes('127.0.0.1'));

  if (loading) {
    return <div className="p-6"><p className="text-sm text-[#3D4F5F]">Scanning network...</p></div>;
  }

  return (
    <div className="p-6 space-y-6 max-w-4xl">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-bold text-[#E6EDF3]">Mesh Network</h1>
        <Button size="sm" variant="secondary">
          <Wifi size={14} /> Scan
        </Button>
      </div>

      {/* This node */}
      {selfNode && (
        <Card header="This Node">
          <div className="flex items-center gap-3">
            <Monitor size={20} className="text-[#00E5E5]" />
            <div>
              <p className="text-sm font-medium text-[#E6EDF3]">{selfNode.display_name}</p>
              <p className="text-[10px] text-[#3D4F5F] font-mono">
                {selfNode.address} &middot; {selfNode.capabilities.join(', ')}
              </p>
            </div>
            <span className="ml-auto inline-flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full
              bg-[#2ECC71]/10 text-[#2ECC71] border border-[#2ECC71]/20">
              <span className="h-1.5 w-1.5 rounded-full bg-[#2ECC71]" />
              Online
            </span>
          </div>
        </Card>
      )}

      {/* Connected nodes */}
      <Card header="Connected Nodes">
        {remoteNodes.length === 0 ? (
          <EmptyState
            icon={<Network size={40} />}
            title="No other nodes found"
            description="Install AgentOS on another PC in your network to create a mesh. Both instances will discover each other automatically via mDNS."
          />
        ) : (
          <div className="space-y-3">
            {remoteNodes.map((node) => (
              <div key={node.node_id} className="flex items-center justify-between py-3 border-b border-[#1A1E26] last:border-0">
                <div className="flex items-center gap-3">
                  <Monitor size={18} className="text-[#C5D0DC]" />
                  <div>
                    <p className="text-sm font-medium text-[#E6EDF3]">{node.display_name}</p>
                    <p className="text-[10px] text-[#3D4F5F] font-mono">{node.address}</p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {node.status === 'online' ? (
                    <Wifi size={14} className="text-[#2ECC71]" />
                  ) : (
                    <WifiOff size={14} className="text-[#E74C3C]" />
                  )}
                  <span className={`text-xs ${node.status === 'online' ? 'text-[#2ECC71]' : 'text-[#E74C3C]'}`}>
                    {node.status}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>

      <p className="text-xs text-[#3D4F5F]">
        Mesh uses mDNS for discovery and WebSocket for communication. Only devices on the same network can connect.
      </p>
    </div>
  );
}
