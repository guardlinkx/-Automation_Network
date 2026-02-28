import { Tray, Menu, BrowserWindow, nativeImage, app } from 'electron';
import * as path from 'path';
import { TunnelManager } from './tunnel';

export function createTray(window: BrowserWindow, tunnel: TunnelManager): Tray {
  const iconPath = path.join(__dirname, '../../resources/icon.ico');
  const tray = new Tray(iconPath);

  const updateMenu = () => {
    const status = tunnel.getStatus();
    const statusLabel = status.connected
      ? `Connected — ${status.server?.city || 'Unknown'}`
      : 'Disconnected';

    const contextMenu = Menu.buildFromTemplate([
      {
        label: 'Califax VPN',
        enabled: false,
      },
      { type: 'separator' },
      {
        label: statusLabel,
        enabled: false,
      },
      { type: 'separator' },
      {
        label: status.connected ? 'Disconnect' : 'Connect',
        click: async () => {
          if (status.connected) {
            await tunnel.disconnect();
          } else {
            window.show();
          }
          updateMenu();
        },
      },
      { type: 'separator' },
      {
        label: 'Show Window',
        click: () => {
          window.show();
          window.focus();
        },
      },
      {
        label: 'Quit',
        click: async () => {
          await tunnel.disconnect();
          app.exit(0);
        },
      },
    ]);

    tray.setContextMenu(contextMenu);
    tray.setToolTip(`Califax VPN — ${statusLabel}`);
  };

  tray.on('double-click', () => {
    window.show();
    window.focus();
  });

  updateMenu();

  // Refresh tray menu periodically
  setInterval(updateMenu, 5000);

  return tray;
}
