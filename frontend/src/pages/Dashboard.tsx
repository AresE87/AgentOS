import { useState, useEffect } from 'react';
import {
  Home,
  BookOpen,
  MessageSquare,
  Network,
  LayoutDashboard,
  BarChart3,
  ShieldCheck,
  Code2,
  Settings,
  Bell,
  PanelLeftClose,
  PanelLeft,
  Clock,
  ThumbsUp,
  Rocket,
} from 'lucide-react';
import { useAgent } from '../hooks/useAgent';
import HomePg from './dashboard/Home';
import Playbooks from './dashboard/Playbooks';
import Chat from './dashboard/Chat';
import Board from './dashboard/Board';
import SettingsPg from './dashboard/Settings';
import Mesh from './dashboard/Mesh';
import Analytics from './dashboard/Analytics';
import Developer from './dashboard/Developer';
import ScheduledTasks from './dashboard/ScheduledTasks';
import FeedbackInsights from './dashboard/FeedbackInsights';
import Operations from './dashboard/Operations';
import Readiness from './dashboard/Readiness';

type Tab =
  | 'home'
  | 'playbooks'
  | 'chat'
  | 'board'
  | 'operations'
  | 'mesh'
  | 'analytics'
  | 'developer'
  | 'triggers'
  | 'settings'
  | 'readiness'
  | 'feedback';

type NavSection = 'core' | 'operate' | 'build' | 'more';

interface NavItem {
  id: Tab;
  label: string;
  icon: typeof Home;
  section: NavSection;
}

const NAV_ITEMS: NavItem[] = [
  { id: 'home', label: 'Home', icon: Home, section: 'core' },
  { id: 'chat', label: 'Chat', icon: MessageSquare, section: 'core' },
  { id: 'playbooks', label: 'Playbooks', icon: BookOpen, section: 'core' },
  { id: 'board', label: 'Board', icon: LayoutDashboard, section: 'core' },
  { id: 'operations', label: 'Operations', icon: ShieldCheck, section: 'operate' },
  { id: 'mesh', label: 'Mesh', icon: Network, section: 'operate' },
  { id: 'analytics', label: 'Analytics', icon: BarChart3, section: 'operate' },
  { id: 'triggers', label: 'Triggers', icon: Clock, section: 'operate' },
  { id: 'developer', label: 'Developer', icon: Code2, section: 'build' },
  { id: 'settings', label: 'Settings', icon: Settings, section: 'build' },
  { id: 'readiness', label: 'Readiness', icon: Rocket, section: 'more' },
  { id: 'feedback', label: 'Feedback', icon: ThumbsUp, section: 'more' },
];

const SECTION_ORDER: NavSection[] = ['core', 'operate', 'build', 'more'];

const SECTION_LABELS: Record<NavSection, string> = {
  core: 'Core',
  operate: 'Operate',
  build: 'Build',
  more: 'Readiness',
};

interface DashboardProps {
  onResetWizard?: () => void;
}

export default function Dashboard({ onResetWizard }: DashboardProps) {
  const [activeTab, setActiveTab] = useState<Tab>('home');
  const [collapsed, setCollapsed] = useState(false);
  const [notifications] = useState(0);
  const [setupIncomplete, setSetupIncomplete] = useState(false);
  const { getStatus } = useAgent();

  useEffect(() => {
    getStatus()
      .then((status) => {
        if (!status.providers || status.providers.length === 0) {
          setSetupIncomplete(true);
        }
      })
      .catch(() => setSetupIncomplete(true));
  }, []);

  const sidebarWidth = collapsed ? 'w-[52px]' : 'w-[224px]';

  return (
    <div className="flex h-screen bg-bg-primary">
      <aside
        className={`${sidebarWidth} shrink-0 flex flex-col border-r border-[#1A1E26] bg-bg-surface transition-all duration-200 ease-out`}
      >
        <div className="flex items-center justify-between px-3 py-4">
          <div className="flex items-center gap-2 overflow-hidden">
            <div className="h-7 w-7 shrink-0 rounded-lg bg-cyan/20 flex items-center justify-center">
              <svg
                className="h-4 w-4 text-cyan"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={1.5}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09z"
                />
              </svg>
            </div>
            {!collapsed && (
              <div className="overflow-hidden">
                <span className="block text-sm font-bold tracking-wide text-text-primary whitespace-nowrap">
                  AgentOS
                </span>
                <span className="block text-[10px] uppercase tracking-[0.28em] text-[#3D4F5F]">
                  Definitive
                </span>
              </div>
            )}
          </div>
          <button
            onClick={() => setCollapsed((current) => !current)}
            className="shrink-0 p-1 rounded text-text-muted hover:text-text-secondary transition-colors"
            title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {collapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
          </button>
        </div>

        <nav className="flex-1 px-2 space-y-2 overflow-y-auto">
          {SECTION_ORDER.map((section) => {
            const items = NAV_ITEMS.filter((item) => item.section === section);
            if (items.length === 0) {
              return null;
            }

            return (
              <div key={section} className="space-y-0.5">
                {!collapsed && (
                  <p className="px-3 pt-2 text-[9px] uppercase tracking-[0.24em] text-[#3D4F5F]">
                    {SECTION_LABELS[section]}
                  </p>
                )}
                {items.map((item) => {
                  const Icon = item.icon;
                  const isActive = activeTab === item.id;

                  return (
                    <button
                      key={item.id}
                      onClick={() => setActiveTab(item.id)}
                      title={collapsed ? item.label : undefined}
                      className={`w-full flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium
                        transition-all duration-150 ease-out
                        ${
                          isActive
                            ? 'bg-[rgba(0,229,229,0.08)] text-cyan border-l-2 border-cyan'
                            : 'text-text-secondary hover:bg-[rgba(0,229,229,0.04)] hover:text-text-primary border-l-2 border-transparent'
                        }
                        ${collapsed ? 'justify-center px-0' : ''}
                      `}
                    >
                      <Icon size={16} className="shrink-0" />
                      {!collapsed && <span className="truncate">{item.label}</span>}
                    </button>
                  );
                })}
              </div>
            );
          })}
        </nav>

        <div className="px-2 pb-3 space-y-2">
          <button
            className={`w-full flex items-center gap-3 rounded-lg px-3 py-2 text-sm text-text-secondary
              hover:bg-[rgba(0,229,229,0.04)] hover:text-text-primary transition-all duration-150 ease-out
              ${collapsed ? 'justify-center px-0' : ''}`}
            title="Notifications"
          >
            <div className="relative shrink-0">
              <Bell size={16} />
              {notifications > 0 && (
                <span className="absolute -top-1.5 -right-1.5 h-4 min-w-[16px] rounded-full bg-error text-[9px] font-bold text-white flex items-center justify-center px-1">
                  {notifications}
                </span>
              )}
            </div>
            {!collapsed && <span className="truncate">Notifications</span>}
          </button>

          <div
            className={`flex items-center gap-2 rounded-lg px-3 py-2 ${collapsed ? 'justify-center px-0' : ''}`}
          >
            <div className="h-2 w-2 shrink-0 rounded-full bg-cyan status-working" />
            {!collapsed && (
              <span className="text-[11px] text-text-muted whitespace-nowrap">Agent: Online</span>
            )}
          </div>

          {!collapsed && (
            <div className="px-3 py-1 text-[10px] font-mono text-text-muted">shell: definitive mode</div>
          )}
        </div>
      </aside>

      <main className="flex-1 overflow-y-auto">
        {setupIncomplete && (
          <div className="mx-6 mt-4 flex items-center justify-between rounded-lg border border-[#F39C12]/30 bg-[#F39C12]/10 px-4 py-3">
            <div className="flex items-center gap-3">
              <span className="text-[#F39C12] text-sm font-medium">Setup incomplete</span>
              <span className="text-[#C5D0DC] text-xs">
                No AI providers configured. The agent needs at least one API key to work.
              </span>
            </div>
            {onResetWizard && (
              <button
                onClick={onResetWizard}
                className="shrink-0 rounded-lg border border-[#F39C12]/40 px-3 py-1.5 text-xs font-medium text-[#F39C12] hover:bg-[#F39C12]/20 transition-colors"
              >
                Run Setup
              </button>
            )}
          </div>
        )}
        {activeTab === 'home' && <HomePg />}
        {activeTab === 'playbooks' && <Playbooks />}
        {activeTab === 'chat' && <Chat />}
        {activeTab === 'board' && <Board />}
        {activeTab === 'operations' && <Operations />}
        {activeTab === 'mesh' && <Mesh />}
        {activeTab === 'analytics' && <Analytics />}
        {activeTab === 'developer' && <Developer />}
        {activeTab === 'triggers' && <ScheduledTasks />}
        {activeTab === 'settings' && <SettingsPg onResetWizard={onResetWizard} />}
        {activeTab === 'readiness' && <Readiness />}
        {activeTab === 'feedback' && <FeedbackInsights />}
      </main>
    </div>
  );
}
