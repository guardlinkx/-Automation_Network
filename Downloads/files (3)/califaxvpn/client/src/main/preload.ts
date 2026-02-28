import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('califaxVPN', {
  connect: (config: { licenseKey: string; deviceId?: string; serverRegion?: string }) =>
    ipcRenderer.invoke('vpn:connect', config),
  disconnect: () => ipcRenderer.invoke('vpn:disconnect'),
  getStatus: () => ipcRenderer.invoke('vpn:status'),
  getServers: () => ipcRenderer.invoke('vpn:servers'),
  activateLicense: (key: string) => ipcRenderer.invoke('vpn:activate-license', key),
  minimizeWindow: () => ipcRenderer.invoke('window:minimize'),
  closeWindow: () => ipcRenderer.invoke('window:close'),
  // Next-gen IPC methods
  getPqcStatus: () => ipcRenderer.invoke('pqc:status'),
  generatePqcKeypair: () => ipcRenderer.invoke('pqc:generateKeypair'),
  getMeshStatus: () => ipcRenderer.invoke('mesh:status'),
  getMeshCircuits: () => ipcRenderer.invoke('mesh:circuits'),
  buildMeshCircuit: (hopCount: number) => ipcRenderer.invoke('mesh:buildCircuit', hopCount),
  getAiStatus: () => ipcRenderer.invoke('ai:status'),
  getProtocolRecommendation: (conditions: Record<string, unknown>) => ipcRenderer.invoke('ai:recommendProtocol', conditions),
  getCanaryStatus: () => ipcRenderer.invoke('canary:status'),
  getLocationPolicies: (deviceId: string) => ipcRenderer.invoke('location:policies', deviceId),
});
