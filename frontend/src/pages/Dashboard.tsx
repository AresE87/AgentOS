// AgentOS Dashboard Shell — Redesigned with animations, top bar, keyboard shortcuts
import { useState, useEffect, useCallback, useRef } from 'react';
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
  ThumbsUp,
  HandHelping,
  Search,
  Plus,
  X,
} from 'lucide-react';
import { useAgent } from '../hooks/useAgent';
import HomePg from './dashboard/Home';
import Playbooks from './dashboard/Playbooks';
import Chat from './dashboard/Chat';
import CommandCenter from './dashboard/CommandCenter';
import SettingsPg from './dashboard/Settings';
import Mesh from './dashboard/Mesh';
import Analytics from './dashboard/Analytics';
import Developer from './dashboard/Developer';
import ScheduledTasks from './dashboard/ScheduledTasks';
import FeedbackInsights from './dashboard/FeedbackInsights';
import Handoffs from './dashboard/Handoffs';

/* ------------------------------------------------------------------ */
/*  Types                                                              */
/* ------------------------------------------------------------------ */

type Tab =
  | 'home'
  | 'playbooks'
  | 'chat'
  | 'board'
  | 'mesh'
  | 'analytics'
  | 'developer'
  | 'triggers'
  | 'settings'
  | 'feedback'
  | 'handoffs';

interface NavItem {
  id: Tab;
  label: string;
  icon: typeof Home;
  section: 'main' | 'more';
  shortcut?: string; // keyboard shortcut hint
}

const NAV_ITEMS: NavItem[] = [
  { id: 'home', label: 'Home', icon: Home, section: 'main', shortcut: '1' },
  { id: 'chat', label: 'Chat', icon: MessageSquare, section: 'main', shortcut: '2' },
  { id: 'board', label: 'Command', icon: LayoutDashboard, section: 'main', shortcut: '3' },
  { id: 'playbooks', label: 'Playbooks', icon: BookOpen, section: 'main', shortcut: '4' },
  { id: 'mesh', label: 'Mesh', icon: Network, section: 'main', shortcut: '5' },
  { id: 'analytics', label: 'Analytics', icon: BarChart3, section: 'main', shortcut: '6' },
  { id: 'developer', label: 'Developer', icon: Code2, section: 'main', shortcut: '7' },
  { id: 'triggers', label: 'Triggers', icon: Clock, section: 'main', shortcut: '8' },
  { id: 'settings', label: 'Settings', icon: Settings, section: 'main', shortcut: '9' },
  { id: 'feedback', label: 'Feedback', icon: ThumbsUp, section: 'more' },
  { id: 'handoffs', label: 'Handoffs', icon: HandHelping, section: 'more' },
];

const TAB_LABELS: Record<Tab, string> = {
  home: 'Home',
  chat: 'Chat',
  board: 'Command Center',
  playbooks: 'Playbooks',
  mesh: 'Agent Mesh',
  analytics: 'Analytics',
  developer: 'Developer Tools',
  triggers: 'Scheduled Tasks',
  settings: 'Settings',
  feedback: 'Feedback & Insights',
  handoffs: 'Escalation Handoffs',
};

/* ------------------------------------------------------------------ */
/*  Component                                                          */
/* ------------------------------------------------------------------ */

interface DashboardProps {
  onResetWizard?: () => void;
}

export default function Dashboard({ onResetWizard }: DashboardProps) {
  const [activeTab, setActiveTab] = useState<Tab>('home');
  const [collapsed, setCollapsed] = useState(false);
  const [notifications] = useState(0);
  const [setupIncomplete, setSetupIncomplete] = useState(false);
  const [agentState, setAgentState] = useState<'idle' | 'working' | 'error'>('idle');
  const [agentTask, setAgentTask] = useState('');
  const [searchOpen, setSearchOpen] = useState(false);
  const [pageTransition, setPageTransition] = useState(true);
  const searchRef = useRef<HTMLInputElement>(null);
  const { getStatus, getPendingShellInvocation } = useAgent();

  // Check setup status
  useEffect(() => {
    getStatus()
      .then((status) => {
        if (!status.providers || status.providers.length === 0) {
          setSetupIncomplete(true);
        }
        if (status.state === 'running') {
          setAgentState('working');
          setAgentTask('Processing...');
        }
      })
      .catch(() => setSetupIncomplete(true));
  }, []);

  // Check for pending shell invocations
  useEffect(() => {
    getPendingShellInvocation()
      .then((inv) => { if (inv) setActiveTab('developer'); })
      .catch(() => undefined);
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Cmd/Ctrl+K → search
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setSearchOpen((o) => !o);
        setTimeout(() => searchRef.current?.focus(), 100);
        return;
      }
      // Escape → close modals/search
      if (e.key === 'Escape') {
        setSearchOpen(false);
        return;
      }
      // Number keys 1-9 → navigate (only if not in input)
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
      const num = parseInt(e.key);
      if (num >= 1 && num <= 9) {
        const item = NAV_ITEMS.find((n) => n.shortcut === e.key);
        if (item) {
          e.preventDefault();
          switchTab(item.id);
        }
      }
      // Cmd+N → new chat
      if ((e.metaKey || e.ctrlKey) && e.key === 'n') {
        e.preventDefault();
        switchTab('chat');
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  // Page transition animation
  const switchTab = useCallback((tab: Tab) => {
    setPageTransition(false);
    setTimeout(() => {
      setActiveTab(tab);
      setPageTransition(true);
    }, 100);
  }, []);

  // Listen for agent state changes
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen<any>('agent:task_completed', () => {
          setAgentState('idle');
          setAgentTask('');
        });
      } catch { /* not in Tauri */ }
    })();
    return () => { if (unlisten) unlisten(); };
  }, []);

  const sidebarW = collapsed ? 'w-[52px]' : 'w-[210px]';

  return (
    <div className="flex h-screen bg-bg-primary">
      {/* ─── SIDEBAR ─── */}
      <aside
        className={`${sidebarW} shrink-0 flex flex-col border-r border-[#1A1E26] bg-bg-surface transition-all duration-200 ease-out`}
      >
        {/* Logo */}
        <div className="flex items-center justify-between px-3 py-4">
          <div className="flex items-center gap-2 overflow-hidden">
            <div
              className="h-7 w-7 shrink-0 rounded-lg bg-cyan/20 flex items-center justify-center"
              style={{ boxShadow: '0 0 12px rgba(0,229,229,0.15)' }}
            >
              <svg className="h-4 w-4 text-cyan" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09z" />
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
          >
            {collapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
          </button>
        </div>

        {/* Nav items */}
        <nav className="flex-1 px-2 space-y-0.5 overflow-y-auto">
          {NAV_ITEMS.filter((i) => i.section === 'main').map((item) => {
            const Icon = item.icon;
            const isActive = activeTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => switchTab(item.id)}
                title={collapsed ? item.label : undefined}
                className={`w-full flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-150 ease-out group relative
                  ${isActive
                    ? 'bg-[rgba(0,229,229,0.08)] text-cyan border-l-2 border-cyan'
                    : 'text-text-secondary hover:bg-[rgba(0,229,229,0.04)] hover:text-text-primary border-l-2 border-transparent'}
                  ${collapsed ? 'justify-center px-0' : ''}`}
              >
                <Icon size={16} className="shrink-0" />
                {!collapsed && (
                  <>
                    <span className="truncate">{item.label}</span>
                    {item.shortcut && (
                      <span className="ml-auto text-[10px] font-mono text-text-dim opacity-0 group-hover:opacity-100 transition-opacity">
                        {item.shortcut}
                      </span>
                    )}
                  </>
                )}
              </button>
            );
          })}

          {/* More divider */}
          <div className="pt-2 pb-1">
            <div className="border-t border-[#1A1E26]" />
            {!collapsed && (
              <p className="text-[9px] uppercase tracking-widest text-text-dim mt-2 px-3 font-mono">More</p>
            )}
          </div>

          {NAV_ITEMS.filter((i) => i.section === 'more').map((item) => {
            const Icon = item.icon;
            const isActive = activeTab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => switchTab(item.id)}
                title={collapsed ? item.label : undefined}
                className={`w-full flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-150 ease-out
                  ${isActive
                    ? 'bg-[rgba(0,229,229,0.08)] text-cyan border-l-2 border-cyan'
                    : 'text-text-secondary hover:bg-[rgba(0,229,229,0.04)] hover:text-text-primary border-l-2 border-transparent'}
                  ${collapsed ? 'justify-center px-0' : ''}`}
              >
                <Icon size={16} className="shrink-0" />
                {!collapsed && <span className="truncate">{item.label}</span>}
              </button>
            );
          })}
        </nav>

        {/* Bottom */}
        <div className="px-2 pb-3 space-y-2">
          {/* Notifications */}
          <button
            className={`w-full flex items-center gap-3 rounded-lg px-3 py-2 text-sm text-text-secondary hover:bg-[rgba(0,229,229,0.04)] hover:text-text-primary transition-all duration-150 ${collapsed ? 'justify-center px-0' : ''}`}
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
          <div className={`flex items-center gap-2 rounded-lg px-3 py-2 ${collapsed ? 'justify-center px-0' : ''}`}>
            <div
              className={`h-2 w-2 shrink-0 rounded-full ${
                agentState === 'working' ? 'bg-cyan status-working' :
                agentState === 'error' ? 'bg-error' : 'bg-success'
              }`}
            />
            {!collapsed && (
              <span className="text-[11px] text-text-muted truncate">
                {agentState === 'working' ? `Working: ${agentTask.slice(0, 20)}...` :
                 agentState === 'error' ? 'Agent: Error' : 'Agent: Idle'}
              </span>
            )}
          </div>

          {!collapsed && (
            <div className="px-3 py-1 text-[10px] font-mono text-text-dim">v0.1.0</div>
          )}
        </div>
      </aside>

      {/* ─── MAIN AREA ─── */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Top Bar */}
        <header className="h-12 shrink-0 flex items-center justify-between px-6 border-b border-[#1A1E26] bg-bg-surface/50 backdrop-blur-sm">
          {/* Breadcrumb */}
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-text-primary">{TAB_LABELS[activeTab]}</span>
          </div>

          {/* Search */}
          <button
            onClick={() => { setSearchOpen(true); setTimeout(() => searchRef.current?.focus(), 100); }}
            className="flex items-center gap-2 rounded-lg border border-[rgba(0,229,229,0.08)] bg-bg-primary/50 px-3 py-1.5 text-xs text-text-muted hover:border-[rgba(0,229,229,0.15)] transition-all w-64"
          >
            <Search size={12} />
            <span>Search tasks, playbooks, agents...</span>
            <span className="ml-auto text-[10px] font-mono text-text-dim border border-[rgba(0,229,229,0.1)] rounded px-1">
              {navigator.platform.includes('Mac') ? '\u2318K' : 'Ctrl+K'}
            </span>
          </button>

          {/* Right side */}
          <div className="flex items-center gap-3">
            <button
              onClick={() => switchTab('chat')}
              className="flex items-center gap-1.5 rounded-lg bg-cyan/10 border border-cyan/20 px-3 py-1.5 text-xs text-cyan hover:bg-cyan/15 transition-all"
            >
              <Plus size={12} /> New Task
            </button>
          </div>
        </header>

        {/* Search overlay */}
        {searchOpen && (
          <div
            className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]"
            onClick={() => setSearchOpen(false)}
          >
            <div className="absolute inset-0 bg-bg-primary/80 backdrop-blur-sm" />
            <div
              className="relative w-full max-w-lg bg-bg-surface border border-[rgba(0,229,229,0.15)] rounded-xl shadow-2xl overflow-hidden"
              onClick={(e) => e.stopPropagation()}
              style={{ boxShadow: '0 0 40px rgba(0,229,229,0.08)' }}
            >
              <div className="flex items-center gap-3 px-4 py-3 border-b border-[#1A1E26]">
                <Search size={16} className="text-text-muted" />
                <input
                  ref={searchRef}
                  type="text"
                  placeholder="Search tasks, playbooks, agents..."
                  className="flex-1 bg-transparent text-sm text-text-primary placeholder-text-muted outline-none"
                  autoFocus
                />
                <button onClick={() => setSearchOpen(false)} className="text-text-muted hover:text-text-secondary">
                  <X size={14} />
                </button>
              </div>
              <div className="p-4 text-center text-xs text-text-muted">
                Type to search across all tasks, playbooks, and agents
              </div>
            </div>
          </div>
        )}

        {/* Setup banner */}
        {setupIncomplete && (
          <div className="mx-6 mt-4 flex items-center justify-between rounded-lg border border-warning/30 bg-warning/10 px-4 py-3">
            <div className="flex items-center gap-3">
              <span className="text-warning text-sm font-medium">Setup incomplete</span>
              <span className="text-text-secondary text-xs">No AI providers configured.</span>
            </div>
            {onResetWizard && (
              <button onClick={onResetWizard} className="shrink-0 rounded-lg border border-warning/40 px-3 py-1.5 text-xs font-medium text-warning hover:bg-warning/20 transition-colors">
                Run Setup
              </button>
            )}
          </div>
        )}

        {/* Agent working bar */}
        {agentState === 'working' && (
          <div className="h-1 w-full bg-bg-deep overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-cyan to-cyan-dark"
              style={{
                animation: 'progress-slide 2s ease-in-out infinite',
                width: '40%',
              }}
            />
          </div>
        )}

        {/* Page content with transition */}
        <main
          className="flex-1 overflow-y-auto"
          style={{
            opacity: pageTransition ? 1 : 0,
            transform: pageTransition ? 'translateY(0)' : 'translateY(4px)',
            transition: 'opacity 200ms ease-out, transform 200ms ease-out',
          }}
        >
          {activeTab === 'home' && <HomePg />}
          {activeTab === 'playbooks' && <Playbooks />}
          {activeTab === 'chat' && <Chat />}
          {activeTab === 'board' && <CommandCenter />}
          {activeTab === 'mesh' && <Mesh />}
          {activeTab === 'analytics' && <Analytics />}
          {activeTab === 'developer' && <Developer />}
          {activeTab === 'triggers' && <ScheduledTasks />}
          {activeTab === 'settings' && <SettingsPg onResetWizard={onResetWizard} />}
          {activeTab === 'feedback' && <FeedbackInsights />}
          {activeTab === 'handoffs' && <Handoffs />}
        </main>
      </div>
    </div>
  );
}
