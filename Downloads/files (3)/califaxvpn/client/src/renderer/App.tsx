import React, { useState, useEffect, useCallback } from 'react';
import Login from './components/Login';
import Dashboard from './components/Dashboard';
import Servers from './components/Servers';
import Settings from './components/Settings';

type View = 'login' | 'dashboard' | 'servers' | 'settings';

interface VpnStatus {
  connected: boolean;
  sessionId: number | null;
  connectedAt: string | null;
  licenseKey: string | null;
  hasLicense: boolean;
  server: { region: string; country: string; city: string } | null;
  killSwitch: boolean;
}

export default function App() {
  const [view, setView] = useState<View>('login');
  const [status, setStatus] = useState<VpnStatus>({
    connected: false,
    sessionId: null,
    connectedAt: null,
    licenseKey: null,
    hasLicense: false,
    server: null,
    killSwitch: false,
  });
  const [selectedRegion, setSelectedRegion] = useState<string>('');

  const refreshStatus = useCallback(async () => {
    const s = await window.califaxVPN.getStatus();
    setStatus(s);
    if (s.hasLicense && view === 'login') {
      setView('dashboard');
    }
  }, [view]);

  useEffect(() => {
    refreshStatus();
    const interval = setInterval(refreshStatus, 3000);
    return () => clearInterval(interval);
  }, [refreshStatus]);

  const onLicenseActivated = () => {
    refreshStatus();
    setView('dashboard');
  };

  return (
    <div className="h-screen flex flex-col bg-califax-bg">
      {/* Title bar */}
      <div className="flex items-center justify-between px-4 py-2 bg-califax-surface border-b border-califax-border">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-califax-accent" />
          <span className="text-sm font-semibold tracking-wide">CALIFAX VPN</span>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => window.califaxVPN.minimizeWindow()}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-califax-border text-califax-muted hover:text-califax-text transition-colors"
          >
            &#8211;
          </button>
          <button
            onClick={() => window.califaxVPN.closeWindow()}
            className="w-6 h-6 flex items-center justify-center rounded hover:bg-red-500/20 text-califax-muted hover:text-red-400 transition-colors"
          >
            &#10005;
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {view === 'login' && <Login onActivated={onLicenseActivated} />}
        {view === 'dashboard' && (
          <Dashboard
            status={status}
            selectedRegion={selectedRegion}
            onRefresh={refreshStatus}
          />
        )}
        {view === 'servers' && (
          <Servers
            selectedRegion={selectedRegion}
            onSelect={(region) => {
              setSelectedRegion(region);
              setView('dashboard');
            }}
          />
        )}
        {view === 'settings' && <Settings status={status} />}
      </div>

      {/* Bottom nav */}
      {status.hasLicense && (
        <div className="flex border-t border-califax-border bg-califax-surface">
          {(['dashboard', 'servers', 'settings'] as const).map((v) => (
            <button
              key={v}
              onClick={() => setView(v)}
              className={`flex-1 py-3 text-xs font-medium uppercase tracking-wider transition-colors ${
                view === v
                  ? 'text-califax-accent border-t-2 border-califax-accent'
                  : 'text-califax-muted hover:text-califax-text'
              }`}
            >
              {v === 'dashboard' ? 'Connect' : v === 'servers' ? 'Servers' : 'Settings'}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
