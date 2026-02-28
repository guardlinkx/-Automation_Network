import React, { useState } from 'react';

interface SettingsProps {
  status: {
    killSwitch: boolean;
    licenseKey: string | null;
  };
}

export default function Settings({ status }: SettingsProps) {
  const [killSwitch, setKillSwitch] = useState(status.killSwitch);
  const [dnsProtection, setDnsProtection] = useState(true);
  const [autoConnect, setAutoConnect] = useState(false);
  const [startWithWindows, setStartWithWindows] = useState(false);

  return (
    <div className="p-4">
      <h2 className="text-lg font-bold mb-4">Settings</h2>

      <div className="space-y-1">
        {/* Security section */}
        <p className="text-xs font-medium text-califax-muted uppercase tracking-wider mb-2 mt-4">
          Security
        </p>

        <ToggleRow
          label="Kill Switch"
          description="Block internet if VPN drops unexpectedly"
          enabled={killSwitch}
          onChange={setKillSwitch}
        />

        <ToggleRow
          label="DNS Leak Protection"
          description="Force DNS through encrypted tunnel"
          enabled={dnsProtection}
          onChange={setDnsProtection}
        />

        {/* General section */}
        <p className="text-xs font-medium text-califax-muted uppercase tracking-wider mb-2 mt-6">
          General
        </p>

        <ToggleRow
          label="Auto-Connect"
          description="Connect when app starts"
          enabled={autoConnect}
          onChange={setAutoConnect}
        />

        <ToggleRow
          label="Start with Windows"
          description="Launch at system startup"
          enabled={startWithWindows}
          onChange={setStartWithWindows}
        />

        {/* Account section */}
        <p className="text-xs font-medium text-califax-muted uppercase tracking-wider mb-2 mt-6">
          Account
        </p>

        <div className="p-3 bg-califax-surface rounded-lg border border-califax-border">
          <p className="text-sm text-califax-muted">License Key</p>
          <p className="font-mono text-sm mt-1">
            {status.licenseKey || 'Not activated'}
          </p>
        </div>

        {/* About section */}
        <p className="text-xs font-medium text-califax-muted uppercase tracking-wider mb-2 mt-6">
          About
        </p>

        <div className="p-3 bg-califax-surface rounded-lg border border-califax-border space-y-1">
          <div className="flex justify-between text-sm">
            <span className="text-califax-muted">Version</span>
            <span>1.0.0</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-califax-muted">Protocol</span>
            <span>Califax Secure Tunnel</span>
          </div>
          <div className="flex justify-between text-sm">
            <span className="text-califax-muted">Encryption</span>
            <span>ChaCha20-Poly1305</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function ToggleRow({
  label,
  description,
  enabled,
  onChange,
}: {
  label: string;
  description: string;
  enabled: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between p-3 bg-califax-surface rounded-lg border border-califax-border">
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-califax-muted">{description}</p>
      </div>
      <button
        onClick={() => onChange(!enabled)}
        className={`relative w-11 h-6 rounded-full transition-colors ${
          enabled ? 'bg-califax-accent' : 'bg-califax-border'
        }`}
      >
        <div
          className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow transition-transform ${
            enabled ? 'translate-x-[22px]' : 'translate-x-0.5'
          }`}
        />
      </button>
    </div>
  );
}
