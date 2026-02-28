interface CalifaxVPN {
  connect(config: { licenseKey: string; deviceId?: string; serverRegion?: string }): Promise<{
    success: boolean;
    error?: string;
    server?: { region: string; country: string; city: string };
  }>;
  disconnect(): Promise<{ success: boolean; error?: string }>;
  getStatus(): Promise<{
    connected: boolean;
    sessionId: number | null;
    connectedAt: string | null;
    licenseKey: string | null;
    hasLicense: boolean;
    server: { region: string; country: string; city: string } | null;
    killSwitch: boolean;
  }>;
  getServers(): Promise<{
    servers: Array<{
      id: number;
      region: string;
      country: string;
      city: string;
      ip_address: string;
      is_active: boolean;
      max_peers: number;
      current_load: number;
      load_percent: number;
    }>;
  }>;
  activateLicense(key: string): Promise<{ valid: boolean; error?: string }>;
  minimizeWindow(): Promise<void>;
  closeWindow(): Promise<void>;
}

declare global {
  interface Window {
    califaxVPN: CalifaxVPN;
  }
}

export {};
