import { useState } from 'react';
import {
  Home,
  BookOpen,
  MessageSquare,
  Network,
  LayoutDashboard,
  BarChart3,
  Code2,
  Settings,
  Bell,
  PanelLeftClose,
  PanelLeft,
  Clock,
  Video,
  ThumbsUp,
} from 'lucide-react';
import HomePg from './dashboard/Home';
import Playbooks from './dashboard/Playbooks';
import Chat from './dashboard/Chat';
import Board from './dashboard/Board';
import SettingsPg from './dashboard/Settings';
import Mesh from './dashboard/Mesh';
import Analytics from './dashboard/Analytics';
import Developer from './dashboard/Developer';
import ScheduledTasks from './dashboard/ScheduledTasks';
import StepRecorder from './dashboard/StepRecorder';
import FeedbackInsights from './dashboard/FeedbackInsights';

type Tab = 'home' | 'playbooks' | 'chat' | 'board' | 'mesh' | 'analytics' | 'developer' | 'triggers' | 'settings' | 'recorder' | 'feedback';

interface NavItem {
  id: Tab;
  label: string;
  icon: typeof Home;
  section?: 'main' | 'more';
}

const NAV_ITEMS: NavItem[] = [
  { id: 'home', label: 'Home', icon: Home, section: 'main' },
  { id: 'playbooks', label: 'Playbooks', icon: BookOpen, section: 'main' },
  { id: 'chat', label: 'Chat', icon: MessageSquare, section: 'main' },
  { id: 'board', label: 'Board', icon: LayoutDashboard, section: 'main' },
  { id: 'mesh', label: 'Mesh', icon: Network, section: 'main' },
  { id: 'analytics', label: 'Analytics', icon: BarChart3, section: 'main' },
  { id: 'developer', label: 'Developer', icon: Code2, section: 'main' },
  { id: 'triggers', label: 'Triggers', icon: Clock, section: 'main' },
  { id: 'settings', label: 'Settings', icon: Settings, section: 'main' },
  // More section
  { id: 'recorder', label: 'Recorder', icon: Video, section: 'more' },
  { id: 'feedback', label: 'Feedback', icon: ThumbsUp, section: 'more' },
];

interface DashboardProps {
  onResetWizard?: () => void;
}

export default function Dashboard({ onResetWizard }: DashboardProps) {
  const [activeTab, setActiveTab] = useState<Tab>('home');
  const [collapsed, setCollapsed] = useState(false);
  const [notifications] = useState(3);

  const sidebarWidth = collapsed ? 'w-[52px]' : 'w-[210px]';

  return (
    <div className="flex h-screen bg-bg-primary">
      {/* Sidebar */}
      <aside
        className={`${sidebarWidth} shrink-0 flex flex-col border-r border-[#1A1E26] bg-bg-surface transition-all duration-200 ease-out`}
      >
        {/* Logo + collapse toggle */}
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
              <span className="text-sm font-bold text-text-primary tracking-wide whitespace-nowrap">
                AgentOS
              </span>
            )}
          </div>
          <button
            onClick={() => setCollapsed((c) => !c)}
            className="shrink-0 p-1 rounded text-text-muted hover:text-text-secondary transition-colors"
            title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {collapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
          </button>
        </div>

        {/* Navigation */}
        <nav className="flex-1 px-2 space-y-0.5 overflow-y-auto">
          {NAV_ITEMS.filter((i) => i.section === 'main').map((item) => {
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

          {/* More section divider */}
          <div className="pt-2 pb-1">
            <div className="border-t border-[#1A1E26]" />
            {!collapsed && (
              <p className="text-[9px] uppercase tracking-widest text-[#3D4F5F] mt-2 px-3">More</p>
            )}
          </div>

          {NAV_ITEMS.filter((i) => i.section === 'more').map((item) => {
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
        </nav>

        {/* Bottom section */}
        <div className="px-2 pb-3 space-y-2">
          {/* Notification bell */}
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

          {/* Agent status */}
          <div
            className={`flex items-center gap-2 rounded-lg px-3 py-2 ${collapsed ? 'justify-center px-0' : ''}`}
          >
            <div className="h-2 w-2 shrink-0 rounded-full bg-cyan status-working" />
            {!collapsed && (
              <span className="text-[11px] text-text-muted whitespace-nowrap">Agent: Online</span>
            )}
          </div>

          {/* Version */}
          {!collapsed && (
            <div className="px-3 py-1 text-[10px] font-mono text-text-muted">v0.1.0</div>
          )}
        </div>
      </aside>

      {/* Content */}
      <main className="flex-1 overflow-y-auto">
        {activeTab === 'home' && <HomePg />}
        {activeTab === 'playbooks' && <Playbooks />}
        {activeTab === 'chat' && <Chat />}
        {activeTab === 'board' && <Board />}
        {activeTab === 'mesh' && <Mesh />}
        {activeTab === 'analytics' && <Analytics />}
        {activeTab === 'developer' && <Developer />}
        {activeTab === 'triggers' && <ScheduledTasks />}
        {activeTab === 'settings' && <SettingsPg onResetWizard={onResetWizard} />}
        {activeTab === 'recorder' && <StepRecorder />}
        {activeTab === 'feedback' && <FeedbackInsights />}
      </main>
    </div>
  );
}
