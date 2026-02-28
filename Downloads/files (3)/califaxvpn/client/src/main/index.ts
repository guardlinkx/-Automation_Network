import { app, BrowserWindow, ipcMain, Tray, Menu, nativeImage } from 'electron';
import * as path from 'path';
import { TunnelManager } from './tunnel';
import { createTray } from './tray';

let mainWindow: BrowserWindow | null = null;
let tray: Tray | null = null;
const tunnel = new TunnelManager();

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 420,
    height: 680,
    resizable: false,
    frame: false,
    transparent: false,
    backgroundColor: '#0a0a0f',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
    icon: path.join(__dirname, '../../resources/icon.ico'),
    title: 'Califax VPN',
  });

  if (process.env.NODE_ENV === 'development') {
    mainWindow.loadURL('http://localhost:5173');
  } else {
    mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
  }

  mainWindow.on('close', (e) => {
    if (tray) {
      e.preventDefault();
      mainWindow?.hide();
    }
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

app.whenReady().then(() => {
  createWindow();
  tray = createTray(mainWindow!, tunnel);

  // IPC Handlers
  ipcMain.handle('vpn:connect', async (_event, config: {
    licenseKey: string;
    deviceId: string;
    serverRegion?: string;
  }) => {
    return tunnel.connect(config);
  });

  ipcMain.handle('vpn:disconnect', async () => {
    return tunnel.disconnect();
  });

  ipcMain.handle('vpn:status', async () => {
    return tunnel.getStatus();
  });

  ipcMain.handle('vpn:servers', async () => {
    return tunnel.getServers();
  });

  ipcMain.handle('vpn:activate-license', async (_event, licenseKey: string) => {
    return tunnel.activateLicense(licenseKey);
  });

  ipcMain.handle('window:minimize', () => {
    mainWindow?.minimize();
  });

  ipcMain.handle('window:close', () => {
    mainWindow?.hide();
  });
});

app.on('window-all-closed', () => {
  // Keep running in tray on Windows
});

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow();
  }
});

app.on('before-quit', async () => {
  await tunnel.disconnect();
  tray?.destroy();
});
