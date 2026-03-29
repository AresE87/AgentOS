// AOS-R2 — Developer tools with Vision E2E test panel
import { useState } from 'react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import {
  Camera,
  Eye,
  MousePointer,
  Keyboard,
  Terminal,
} from 'lucide-react';

export default function Developer() {
  const [log, setLog] = useState<string[]>([]);
  const [loading, setLoading] = useState<string | null>(null);

  const addLog = (msg: string) => setLog((prev) => [...prev.slice(-20), `[${new Date().toLocaleTimeString()}] ${msg}`]);

  const callBackend = async (cmd: string, args?: Record<string, unknown>) => {
    setLoading(cmd);
    try {
      // Dynamic import to access invoke directly
      const isTauri = '__TAURI_INTERNALS__' in window || '__TAURI__' in window;
      if (!isTauri) {
        addLog(`[MOCK] ${cmd} — requires Tauri backend`);
        setLoading(null);
        return null;
      }
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<any>(`cmd_${cmd}`, args);
      addLog(`${cmd}: OK`);
      setLoading(null);
      return result;
    } catch (e: any) {
      addLog(`${cmd}: ERROR — ${e?.message || e}`);
      setLoading(null);
      return null;
    }
  };

  const handleCapture = async () => {
    const result = await callBackend('capture_screenshot');
    if (result) {
      addLog(`Screenshot saved: ${result.path}`);
    }
  };

  const handleVision = async () => {
    const result = await callBackend('test_vision');
    if (result) {
      addLog(`Vision analysis (${result.model}):`);
      addLog(result.analysis?.substring(0, 200) + '...');
    }
  };

  const handleClick = async () => {
    const x = prompt('X coordinate:');
    const y = prompt('Y coordinate:');
    if (x && y) {
      await callBackend('test_click', { x: parseInt(x), y: parseInt(y) });
    }
  };

  const handleType = async () => {
    const text = prompt('Text to type:');
    if (text) {
      await callBackend('test_type', { text });
    }
  };

  return (
    <div className="p-6 space-y-6 max-w-5xl">
      <h1 className="text-xl font-bold text-[#E6EDF3]">Developer Tools</h1>

      {/* Vision E2E Test Panel */}
      <Card header="Vision E2E Tests (R2)">
        <p className="text-xs text-[#3D4F5F] mb-4">
          Test individual vision pipeline components. Requires Tauri backend running.
        </p>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="secondary" onClick={handleCapture} loading={loading === 'capture_screenshot'}>
            <Camera size={14} /> Capture Screen
          </Button>
          <Button size="sm" variant="secondary" onClick={handleVision} loading={loading === 'test_vision'}>
            <Eye size={14} /> Vision Analyze
          </Button>
          <Button size="sm" variant="secondary" onClick={handleClick} loading={loading === 'test_click'}>
            <MousePointer size={14} /> Test Click
          </Button>
          <Button size="sm" variant="secondary" onClick={handleType} loading={loading === 'test_type'}>
            <Keyboard size={14} /> Test Type
          </Button>
        </div>
      </Card>

      {/* Log output */}
      <Card header="Log">
        <div className="font-mono text-xs space-y-1 max-h-[400px] overflow-y-auto">
          {log.length === 0 ? (
            <p className="text-[#3D4F5F]">Run a test to see output here.</p>
          ) : (
            log.map((line, i) => (
              <div key={i} className={`${line.includes('ERROR') ? 'text-[#E74C3C]' : line.includes('OK') ? 'text-[#2ECC71]' : 'text-[#C5D0DC]'}`}>
                {line}
              </div>
            ))
          )}
        </div>
      </Card>

      {/* Quick reference */}
      <Card header="IPC Commands">
        <div className="grid grid-cols-2 gap-2 text-xs font-mono text-[#3D4F5F]">
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_capture_screenshot</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_vision</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_click</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_type</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_test_key_combo</div>
          <div className="flex items-center gap-2"><Terminal size={12} /> cmd_run_pc_task</div>
        </div>
      </Card>
    </div>
  );
}
