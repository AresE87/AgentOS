import { Network } from 'lucide-react';

export default function Mesh() {
  return (
    <div className="p-6 flex flex-col items-center justify-center h-full text-center">
      <Network size={48} className="text-[#3D4F5F] mb-4" />
      <h2 className="text-lg font-medium text-[#E6EDF3] mb-2">Mesh Network</h2>
      <p className="text-sm text-[#3D4F5F] max-w-md">
        Connect multiple PCs to distribute work across your network.
        This feature requires at least 2 AgentOS instances running.
      </p>
      <p className="text-xs text-[#2A3441] mt-4">Available in a future update</p>
    </div>
  );
}
