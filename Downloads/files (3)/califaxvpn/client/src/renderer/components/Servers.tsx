import React, { useState, useEffect } from 'react';

interface Server {
  id: number;
  region: string;
  country: string;
  city: string;
  load_percent: number;
  current_load: number;
  max_peers: number;
}

interface ServersProps {
  selectedRegion: string;
  onSelect: (region: string) => void;
}

const FLAG_MAP: Record<string, string> = {
  'United States': '\u{1F1FA}\u{1F1F8}',
  'Germany': '\u{1F1E9}\u{1F1EA}',
  'Japan': '\u{1F1EF}\u{1F1F5}',
  'United Kingdom': '\u{1F1EC}\u{1F1E7}',
  'Canada': '\u{1F1E8}\u{1F1E6}',
};

export default function Servers({ selectedRegion, onSelect }: ServersProps) {
  const [servers, setServers] = useState<Server[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadServers();
  }, []);

  const loadServers = async () => {
    try {
      const data = await window.califaxVPN.getServers();
      setServers(data.servers);
    } catch {
      // Offline or error
    } finally {
      setLoading(false);
    }
  };

  const getLoadColor = (percent: number) => {
    if (percent < 50) return 'bg-califax-accent';
    if (percent < 80) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  return (
    <div className="p-4">
      <h2 className="text-lg font-bold mb-1">Server Locations</h2>
      <p className="text-califax-muted text-sm mb-4">
        Select a server to connect through
      </p>

      {loading ? (
        <div className="flex items-center justify-center py-16">
          <div className="w-8 h-8 border-2 border-califax-accent border-t-transparent rounded-full animate-spin" />
        </div>
      ) : servers.length === 0 ? (
        <div className="text-center py-16 text-califax-muted">
          <p>No servers available</p>
          <button
            onClick={loadServers}
            className="mt-2 text-califax-accent text-sm hover:underline"
          >
            Retry
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          {/* Auto-select option */}
          <button
            onClick={() => onSelect('')}
            className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
              selectedRegion === ''
                ? 'border-califax-accent bg-califax-accent/5'
                : 'border-califax-border bg-califax-surface hover:border-califax-accent/50'
            }`}
          >
            <span className="text-xl">&#9889;</span>
            <div className="flex-1 text-left">
              <p className="font-medium">Fastest Server</p>
              <p className="text-xs text-califax-muted">Auto-select lowest load</p>
            </div>
            {selectedRegion === '' && (
              <div className="w-2 h-2 rounded-full bg-califax-accent" />
            )}
          </button>

          {servers.map((server) => (
            <button
              key={server.id}
              onClick={() => onSelect(server.region)}
              className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                selectedRegion === server.region
                  ? 'border-califax-accent bg-califax-accent/5'
                  : 'border-califax-border bg-califax-surface hover:border-califax-accent/50'
              }`}
            >
              <span className="text-xl">
                {FLAG_MAP[server.country] || '\u{1F310}'}
              </span>
              <div className="flex-1 text-left">
                <p className="font-medium">{server.city}, {server.country}</p>
                <p className="text-xs text-califax-muted">{server.region}</p>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-16 h-1.5 bg-califax-border rounded-full overflow-hidden">
                  <div
                    className={`h-full rounded-full transition-all ${getLoadColor(server.load_percent)}`}
                    style={{ width: `${Math.max(5, server.load_percent)}%` }}
                  />
                </div>
                <span className="text-xs text-califax-muted w-8 text-right">
                  {Math.round(server.load_percent)}%
                </span>
              </div>
              {selectedRegion === server.region && (
                <div className="w-2 h-2 rounded-full bg-califax-accent" />
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
