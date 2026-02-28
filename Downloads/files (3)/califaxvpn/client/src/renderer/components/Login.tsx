import React, { useState } from 'react';

interface LoginProps {
  onActivated: () => void;
}

export default function Login({ onActivated }: LoginProps) {
  const [licenseKey, setLicenseKey] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleActivate = async () => {
    if (!licenseKey.trim()) {
      setError('Please enter your license key');
      return;
    }

    setLoading(true);
    setError('');

    try {
      const result = await window.califaxVPN.activateLicense(licenseKey.trim());
      if (result.valid) {
        onActivated();
      } else {
        setError(result.error || 'Invalid license key');
      }
    } catch (e: any) {
      setError(e.message || 'Activation failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center h-full px-8">
      {/* Logo */}
      <div className="mb-8 text-center">
        <div className="w-20 h-20 mx-auto mb-4 rounded-2xl bg-gradient-to-br from-califax-accent/20 to-califax-accent/5 flex items-center justify-center border border-califax-accent/30">
          <svg className="w-10 h-10 text-califax-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={1.5}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z" />
          </svg>
        </div>
        <h1 className="text-2xl font-bold">Califax VPN</h1>
        <p className="text-califax-muted text-sm mt-1">Enterprise-Grade Protection</p>
      </div>

      {/* License input */}
      <div className="w-full max-w-sm space-y-4">
        <div>
          <label className="block text-xs font-medium text-califax-muted uppercase tracking-wider mb-2">
            License Key
          </label>
          <input
            type="text"
            value={licenseKey}
            onChange={(e) => setLicenseKey(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleActivate()}
            placeholder="XXXX-XXXX-XXXX-XXXX"
            className="w-full px-4 py-3 bg-califax-surface border border-califax-border rounded-lg text-califax-text placeholder-califax-muted/50 focus:outline-none focus:border-califax-accent transition-colors font-mono text-sm"
            disabled={loading}
          />
        </div>

        {error && (
          <p className="text-red-400 text-sm text-center">{error}</p>
        )}

        <button
          onClick={handleActivate}
          disabled={loading}
          className="w-full py-3 bg-califax-accent hover:bg-califax-accent-hover text-califax-bg font-semibold rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {loading ? 'Activating...' : 'Activate License'}
        </button>

        <p className="text-center text-xs text-califax-muted">
          Purchase a license at{' '}
          <span className="text-califax-accent">califaxvpn.guardlinkx.com</span>
        </p>
      </div>
    </div>
  );
}
