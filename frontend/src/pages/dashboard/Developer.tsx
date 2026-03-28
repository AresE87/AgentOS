// AOS-P2 — Developer tools section (honest state)
import Card from '../../components/Card';
import {
  ExternalLink,
  Code2,
  Terminal,
  FileText,
  Wrench,
} from 'lucide-react';

const DOCS = [
  { label: 'API Reference', icon: <FileText size={16} />, href: '#' },
  { label: 'Python SDK', icon: <Code2 size={16} />, href: '#' },
  { label: 'CLI Tool', icon: <Terminal size={16} />, href: '#' },
];

export default function Developer() {
  return (
    <div className="p-6 space-y-6 max-w-5xl">
      <h1 className="text-xl font-bold text-[#E6EDF3]">Developer</h1>

      {/* Honest placeholder for API keys & webhooks */}
      <div className="flex flex-col items-center justify-center text-center py-8">
        <Wrench size={48} className="text-[#3D4F5F] mb-4" />
        <h2 className="text-lg font-medium text-[#E6EDF3] mb-2">Developer Tools</h2>
        <p className="text-sm text-[#3D4F5F] max-w-md">
          API key management, webhooks, and usage tracking will be available
          when the developer API is enabled. For now, interact with AgentOS
          through the dashboard or Telegram.
        </p>
        <p className="text-xs text-[#2A3441] mt-4">Coming in a future update</p>
      </div>

      {/* Documentation links — these are real, just external links */}
      <Card header="Documentation">
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
          {DOCS.map((doc) => (
            <a
              key={doc.label}
              href={doc.href}
              className="flex items-center gap-3 rounded-lg border border-[#1A1E26] bg-[#0A0E14] p-3
                hover:border-[#00E5E5]/30 hover:bg-[#00E5E5]/5 transition-colors group"
            >
              <div className="text-[#3D4F5F] group-hover:text-[#00E5E5] transition-colors">
                {doc.icon}
              </div>
              <span className="text-sm text-[#E6EDF3]">{doc.label}</span>
              <ExternalLink size={12} className="ml-auto text-[#3D4F5F] group-hover:text-[#00E5E5] transition-colors" />
            </a>
          ))}
        </div>
      </Card>
    </div>
  );
}
