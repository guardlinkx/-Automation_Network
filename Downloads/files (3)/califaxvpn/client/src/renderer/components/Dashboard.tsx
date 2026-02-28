import React, { useState, useEffect } from 'react';

interface DashboardProps {
  status: {
    connected: boolean;
    connectedAt: string | null;
    server: { region: string; country: string; city: string } | null;
    licenseKey: string | null;
  };
  selectedRegion: string;
  onRefresh: () => void;
}

export default function Dashboard({ status, selectedRegion, onRefresh }: DashboardProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [elapsed, setElapsed] = useState('00:00:00');

  useEffect(() => {
    if (!status.connected || !status.connectedAt) {
      setElapsed('00:00:00');
      return;
    }

    const update = () => {
      const diff = Date.now() - new Date(status.connectedAt!).getTime();
      const h = Math.floor(diff / 3600000);
      const m = Math.floor((diff % 3600000) / 60000);
      const s = Math.floor((diff % 60000) / 1000);
      setElapsed(
        `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
      );
    };

    update();
    const interval = setInterval(update, 1000);
    return () => clearInterval(interval);
  }, [status.connected, status.connectedAt]);

  const handleToggle = async () => {
    setLoading(true);
    setError('');

    try {
      if (status.connected) {
        const result = await window.califaxVPN.disconnect();
        if (!result.success) {
          setError(result.error || 'Failed to disconnect');
        }
      } else {
        const result = await window.califaxVPN.connect({
          licenseKey: status.licenseKey!,
          serverRegion: selectedRegion || undefined,
        });
        if (!result.success) {
          setError(result.error || 'Failed to connect');
        }
      }
      onRefresh();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center h-full px-8 py-6">
      {/* Connection status ring */}
      <div className="relative mb-8">
        <div
          className={`w-48 h-48 rounded-full border-4 flex items-center justify-center transition-all duration-500 ${
            status.connected
              ? 'border-califax-accent shadow-[0_0_40px_rgba(0,229,160,0.15)]'
              : 'border-califax-border'
          }`}
        >
          <div className="text-center">
            <div
              className={`w-4 h-4 rounded-full mx-auto mb-3 ${
                status.connected ? 'bg-califax-accent animate-pulse' : 'bg-califax-muted'
              }`}
            />
            <p className="text-lg font-bold">
              {status.connected ? 'Protected' : 'Unprotected'}
            </p>
            {status.connected && (
              <p className="text-califax-muted text-sm font-mono mt-1">{elapsed}</p>
            )}
          </div>
        </div>
      </div>

      {/* Server info */}
      {status.connected && status.server && (
        <div className="mb-6 text-center">
          <p className="text-sm text-califax-muted">Connected to</p>
          <p className="text-lg font-semibold">
            {status.server.city}, {status.server.country}
          </p>
          <p className="text-xs text-califax-muted">{status.server.region}</p>
        </div>
      )}

      {!status.connected && selectedRegion && (
        <div className="mb-6 text-center">
          <p className="text-sm text-califax-muted">Selected server</p>
          <p className="font-semibold">{selectedRegion}</p>
        </div>
      )}

      {error && (
        <p className="text-red-400 text-sm mb-4 text-center">{error}</p>
      )}

      {/* Connect/Disconnect button */}
      <button
        onClick={handleToggle}
        disabled={loading}
        className={`w-full max-w-xs py-4 rounded-xl font-bold text-lg transition-all duration-300 disabled:opacity-50 ${
          status.connected
            ? 'bg-red-500/10 border border-red-500/30 text-red-400 hover:bg-red-500/20'
            : 'bg-califax-accent hover:bg-califax-accent-hover text-califax-bg'
        }`}
      >
        {loading
          ? status.connected
            ? 'Disconnecting...'
            : 'Connecting...'
          : status.connected
          ? 'Disconnect'
          : 'Connect'}
      </button>
    </div>
  );
}
