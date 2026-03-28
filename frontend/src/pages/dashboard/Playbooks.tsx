// Playbooks page — shows installed playbooks with activate/deactivate
import { useState, useEffect } from 'react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import { useAgent } from '../../hooks/useAgent';
import type { Playbook } from '../../types/ipc';
import { Store } from 'lucide-react';

const TIER_COLORS: Record<number, string> = {
  1: 'bg-[#2ECC71]/10 text-[#2ECC71]',
  2: 'bg-[#F39C12]/10 text-[#F39C12]',
  3: 'bg-[#E74C3C]/10 text-[#E74C3C]',
};

const PERM_COLORS: Record<string, string> = {
  cli: 'bg-[#00E5E5]/10 text-[#00E5E5]',
  screen: 'bg-[#5865F2]/10 text-[#5865F2]',
  files: 'bg-[#378ADD]/10 text-[#378ADD]',
  network: 'bg-[#F39C12]/10 text-[#F39C12]',
};

export default function Playbooks() {
  const { getPlaybooks, setActivePlaybook } = useAgent();
  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [active, setActive] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchPlaybooks = () => {
    getPlaybooks()
      .then((data) => {
        setPlaybooks(data.playbooks || []);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  };

  useEffect(() => { fetchPlaybooks(); }, []);

  const handleActivate = (name: string) => {
    const pb = playbooks.find((p) => p.name === name);
    if (pb) {
      setActivePlaybook(pb.path).then(() => {
        setActive(name);
      });
    }
  };

  const handleDeactivate = () => {
    setActivePlaybook('').then(() => setActive(null));
  };

  if (loading) {
    return (
      <div className="p-6">
        <h1 className="text-xl font-semibold text-[#E6EDF3] mb-6">Playbooks</h1>
        <p className="text-[#3D4F5F] text-sm">Loading playbooks...</p>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <h1 className="text-xl font-semibold text-[#E6EDF3]">Playbooks</h1>

      {/* Active Playbook */}
      <Card header="Active Playbook">
        {active ? (
          <div className="flex items-center justify-between">
            <div>
              <p className="text-[#E6EDF3] font-medium">{active}</p>
              <p className="text-[#3D4F5F] text-xs mt-1">Currently active</p>
            </div>
            <Button variant="secondary" size="sm" onClick={handleDeactivate}>
              Deactivate
            </Button>
          </div>
        ) : (
          <p className="text-[#3D4F5F] text-sm">No active playbook. Select one below to activate.</p>
        )}
      </Card>

      {/* Installed */}
      <Card header="Installed">
        {playbooks.length === 0 ? (
          <p className="text-[#3D4F5F] text-sm">No playbooks installed.</p>
        ) : (
          <div className="space-y-3">
            {playbooks.map((pb) => (
              <div
                key={pb.name}
                className="flex items-center justify-between py-3 border-b border-[#1A1E26] last:border-0"
              >
                <div>
                  <p className="text-[#E6EDF3] font-medium">{pb.name}</p>
                  <div className="flex gap-2 mt-1">
                    <span className={`text-[10px] px-2 py-0.5 rounded-full font-medium ${TIER_COLORS[pb.tier] || TIER_COLORS[1]}`}>
                      TIER {pb.tier}
                    </span>
                    {pb.permissions.map((p) => (
                      <span
                        key={p}
                        className={`text-[10px] px-2 py-0.5 rounded-full font-medium uppercase ${PERM_COLORS[p] || 'bg-[#1A1E26] text-[#3D4F5F]'}`}
                      >
                        {p}
                      </span>
                    ))}
                  </div>
                </div>
                {active === pb.name ? (
                  <span className="text-[#2ECC71] text-xs font-medium">Active</span>
                ) : (
                  <Button variant="primary" size="sm" onClick={() => handleActivate(pb.name)}>
                    Activate
                  </Button>
                )}
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Marketplace placeholder */}
      <Card header="Marketplace">
        <div className="flex flex-col items-center justify-center py-8 text-center">
          <Store size={40} className="text-[#3D4F5F] mb-3" />
          <p className="text-[#C5D0DC] font-medium">Marketplace coming soon</p>
          <p className="text-[#3D4F5F] text-sm mt-1 max-w-sm">
            Browse and install community playbooks from the AgentOS marketplace.
          </p>
        </div>
      </Card>
    </div>
  );
}
