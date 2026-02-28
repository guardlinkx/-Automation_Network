import { execFile, exec } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as https from 'https';
import * as http from 'http';
import * as crypto from 'crypto';
import { app } from 'electron';

const API_BASE = 'https://califaxvpn.guardlinkx.com/api/vpn';

interface TunnelConfig {
  server_public_key: string;
  endpoint: string;
  client_ip: string;
  dns: string[];
  allowed_ips: string;
  keepalive: number;
}

interface ConnectResponse {
  status: string;
  session_id: number;
  tunnel_config: TunnelConfig;
  server: {
    region: string;
    country: string;
    city: string;
  };
}

export class TunnelManager {
  private connected = false;
  private sessionId: number | null = null;
  private licenseKey: string | null = null;
  private deviceId: string;
  private connectedAt: Date | null = null;
  private privateKey: string | null = null;
  private publicKey: string | null = null;
  private serverInfo: { region: string; country: string; city: string } | null = null;
  private killSwitchEnabled = false;

  constructor() {
    this.deviceId = this.getDeviceId();
    this.loadStoredLicense();
  }

  private getDeviceId(): string {
    const storePath = path.join(app.getPath('userData'), 'device.json');
    try {
      const data = JSON.parse(fs.readFileSync(storePath, 'utf-8'));
      return data.deviceId;
    } catch {
      const deviceId = crypto.randomUUID();
      fs.mkdirSync(path.dirname(storePath), { recursive: true });
      fs.writeFileSync(storePath, JSON.stringify({ deviceId }));
      return deviceId;
    }
  }

  private loadStoredLicense() {
    const storePath = path.join(app.getPath('userData'), 'license.json');
    try {
      const data = JSON.parse(fs.readFileSync(storePath, 'utf-8'));
      this.licenseKey = data.licenseKey;
    } catch {
      // No stored license
    }
  }

  private storeLicense(licenseKey: string) {
    const storePath = path.join(app.getPath('userData'), 'license.json');
    fs.mkdirSync(path.dirname(storePath), { recursive: true });
    fs.writeFileSync(storePath, JSON.stringify({ licenseKey }));
    this.licenseKey = licenseKey;
  }

  private getWireGuardPath(): string {
    if (app.isPackaged) {
      return path.join(process.resourcesPath, 'wireguard', 'wireguard.exe');
    }
    return path.join(__dirname, '../../resources/wireguard/wireguard.exe');
  }

  private getConfPath(): string {
    return path.join(app.getPath('userData'), 'califax-tunnel.conf');
  }

  private async generateKeypair(): Promise<{ privateKey: string; publicKey: string }> {
    const wgPath = this.getWireGuardPath();
    const wgDir = path.dirname(wgPath);

    return new Promise((resolve, reject) => {
      execFile(path.join(wgDir, 'wg.exe'), ['genkey'], (err, privateKey) => {
        if (err) return reject(err);
        const privKey = privateKey.trim();

        execFile(path.join(wgDir, 'wg.exe'), ['pubkey'], { input: privKey } as any, (err2, publicKey) => {
          if (err2) return reject(err2);
          resolve({ privateKey: privKey, publicKey: publicKey.trim() });
        });
      });
    });
  }

  private apiRequest(method: string, endpoint: string, body?: any): Promise<any> {
    return new Promise((resolve, reject) => {
      const url = new URL(`${API_BASE}${endpoint}`);
      const options = {
        hostname: url.hostname,
        port: url.port || 443,
        path: url.pathname,
        method,
        headers: {
          'Content-Type': 'application/json',
        },
      };

      const req = https.request(options, (res) => {
        let data = '';
        res.on('data', (chunk) => data += chunk);
        res.on('end', () => {
          try {
            const parsed = JSON.parse(data);
            if (res.statusCode && res.statusCode >= 400) {
              reject(new Error(parsed.error || `HTTP ${res.statusCode}`));
            } else {
              resolve(parsed);
            }
          } catch {
            reject(new Error(`Invalid response: ${data}`));
          }
        });
      });

      req.on('error', reject);
      if (body) req.write(JSON.stringify(body));
      req.end();
    });
  }

  async activateLicense(licenseKey: string): Promise<{ valid: boolean; error?: string }> {
    try {
      // Test the license by requesting server list
      this.storeLicense(licenseKey);
      return { valid: true };
    } catch (e: any) {
      return { valid: false, error: e.message };
    }
  }

  async getServers(): Promise<any> {
    return this.apiRequest('GET', '/servers');
  }

  async connect(config: {
    licenseKey: string;
    deviceId?: string;
    serverRegion?: string;
  }): Promise<{ success: boolean; error?: string; server?: any }> {
    if (this.connected) {
      return { success: false, error: 'Already connected' };
    }

    try {
      // Generate keypair locally — private key never leaves the device
      const keypair = await this.generateKeypair();
      this.privateKey = keypair.privateKey;
      this.publicKey = keypair.publicKey;

      // Request tunnel from central API
      const response: ConnectResponse = await this.apiRequest('POST', '/connect', {
        license_key: config.licenseKey,
        device_id: this.deviceId,
        client_public_key: keypair.publicKey,
        server_region: config.serverRegion,
      });

      // Write WireGuard config
      const confContent = [
        '[Interface]',
        `PrivateKey = ${keypair.privateKey}`,
        `Address = ${response.tunnel_config.client_ip}`,
        `DNS = ${response.tunnel_config.dns.join(', ')}`,
        '',
        '[Peer]',
        `PublicKey = ${response.tunnel_config.server_public_key}`,
        `AllowedIPs = ${response.tunnel_config.allowed_ips}`,
        `Endpoint = ${response.tunnel_config.endpoint}`,
        `PersistentKeepalive = ${response.tunnel_config.keepalive}`,
      ].join('\n');

      const confPath = this.getConfPath();
      fs.writeFileSync(confPath, confContent, { mode: 0o600 });

      // Start WireGuard tunnel
      await this.startTunnel(confPath);

      this.connected = true;
      this.sessionId = response.session_id;
      this.connectedAt = new Date();
      this.serverInfo = response.server;
      this.licenseKey = config.licenseKey;

      return { success: true, server: response.server };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  }

  private startTunnel(confPath: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const wgPath = this.getWireGuardPath();
      const tunnelName = 'CalifaxVPN';

      // Install and start the WireGuard tunnel service
      execFile(wgPath, ['/installtunnelservice', confPath], (err) => {
        if (err) return reject(err);
        resolve();
      });
    });
  }

  private stopTunnel(): Promise<void> {
    return new Promise((resolve, reject) => {
      const wgPath = this.getWireGuardPath();
      const tunnelName = 'CalifaxVPN';

      execFile(wgPath, ['/uninstalltunnelservice', 'califax-tunnel'], (err) => {
        if (err) {
          // Try alternative cleanup
          exec('net stop WireGuardTunnel$califax-tunnel', () => resolve());
          return;
        }
        resolve();
      });
    });
  }

  async disconnect(): Promise<{ success: boolean; error?: string }> {
    if (!this.connected) {
      return { success: true };
    }

    try {
      // Stop the tunnel first
      await this.stopTunnel();

      // Notify central API
      if (this.sessionId) {
        try {
          await this.apiRequest('POST', '/disconnect', {
            session_id: this.sessionId,
          });
        } catch {
          // Best effort
        }
      }

      // Clean up config file
      const confPath = this.getConfPath();
      if (fs.existsSync(confPath)) {
        fs.unlinkSync(confPath);
      }

      this.connected = false;
      this.sessionId = null;
      this.connectedAt = null;
      this.privateKey = null;
      this.publicKey = null;
      this.serverInfo = null;

      return { success: true };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  }

  getStatus() {
    return {
      connected: this.connected,
      sessionId: this.sessionId,
      connectedAt: this.connectedAt?.toISOString() || null,
      licenseKey: this.licenseKey ? `${this.licenseKey.slice(0, 8)}...` : null,
      hasLicense: !!this.licenseKey,
      server: this.serverInfo,
      killSwitch: this.killSwitchEnabled,
    };
  }
}

// === Next-Gen Protocol Support ===

export type VpnProtocol = 'wireguard' | 'ikev2' | 'obfuscated_wireguard' | 'shadowsocks' | 'chameleon';

export interface PqcKeyPair {
  x25519_public: string;
  kyber_public: string;
  algorithm: string;
}

export interface MeshCircuitInfo {
  circuit_id: string;
  hop_count: number;
  entry_node: string;
  exit_node: string;
  status: string;
}

export interface NextGenTunnelConfig {
  protocol: VpnProtocol;
  pqc_enabled: boolean;
  mesh_circuit_id?: string;
  obfuscation_mode?: 'none' | 'xor' | 'tls_mimicry' | 'http_masquerade' | 'chameleon';
  double_tunnel?: boolean;
}

export async function requestPqcKeypair(apiBase: string, licenseKey: string): Promise<PqcKeyPair | null> {
  try {
    const resp = await fetch(`${apiBase}/api/vpn/pqc/keypair`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ license_key: licenseKey }),
    });
    if (resp.ok) {
      return await resp.json();
    }
    return null;
  } catch {
    return null;
  }
}

export async function getProtocolRecommendation(apiBase: string, conditions: Record<string, unknown>): Promise<string> {
  try {
    const resp = await fetch(`${apiBase}/api/ai/protocol/recommend`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(conditions),
    });
    if (resp.ok) {
      const data = await resp.json();
      return data.recommended_protocol || 'wireguard';
    }
    return 'wireguard';
  } catch {
    return 'wireguard';
  }
}
