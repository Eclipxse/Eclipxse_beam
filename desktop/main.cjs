const { app, BrowserWindow, Menu, shell } = require('electron');
const path = require('node:path');

const APP_ID = 'com.eclipxse.beam';

app.setAppUserModelId(APP_ID);

function createWindow() {
  const window = new BrowserWindow({
    width: 1360,
    height: 920,
    minWidth: 940,
    minHeight: 680,
    show: false,
    title: 'Eclipxse Beam',
    backgroundColor: '#080609',
    autoHideMenuBar: true,
    titleBarStyle: 'hidden',
    titleBarOverlay: {
      color: '#080609',
      symbolColor: '#d9c9b7',
      height: 42,
    },
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      devTools: !app.isPackaged,
    },
  });

  Menu.setApplicationMenu(null);
  window.loadFile(path.join(__dirname, '..', 'dist', 'index.html'));

  window.once('ready-to-show', () => window.show());

  window.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('https://')) void shell.openExternal(url);
    return { action: 'deny' };
  });

  window.webContents.on('will-navigate', (event, url) => {
    if (url.startsWith('file://')) return;
    event.preventDefault();
    if (url.startsWith('https://')) void shell.openExternal(url);
  });
}

app.whenReady().then(() => {
  createWindow();
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});
