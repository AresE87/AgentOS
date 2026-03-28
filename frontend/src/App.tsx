// Part of the AgentOS dashboard (AOS-022 to AOS-027)
import { useEffect, useState } from 'react';
import { useAgent } from './hooks/useAgent';
import Wizard from './pages/Wizard';
import Dashboard from './pages/Dashboard';

type View = 'loading' | 'wizard' | 'dashboard';

function App() {
    const [view, setView] = useState<View>('loading');
    const { getStatus } = useAgent();

    useEffect(() => {
        getStatus()
            .then((status) => {
                // If no providers are configured, show setup wizard
                if (status.providers.length === 0) {
                    setView('wizard');
                } else {
                    setView('dashboard');
                }
            })
            .catch(() => {
                // If backend is not reachable yet, show wizard
                setView('wizard');
            });
    }, []);

    if (view === 'loading') {
        return (
            <div className="flex h-screen items-center justify-center bg-[#0A0E14]">
                <div className="text-center">
                    <div className="mb-4 h-8 w-8 animate-spin rounded-full border-2 border-[#00E5E5] border-t-transparent mx-auto" />
                    <p className="text-[#C5D0DC] text-sm">Starting AgentOS...</p>
                </div>
            </div>
        );
    }

    if (view === 'wizard') {
        return <Wizard onComplete={() => setView('dashboard')} />;
    }

    return <Dashboard onResetWizard={() => setView('wizard')} />;
}

export default App;
