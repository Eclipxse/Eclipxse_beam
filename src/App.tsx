import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import QRCode from 'react-qr-code';
import { BeamLogo } from './components/BeamLogo';
import { Icon } from './components/Icon';
import { useBeamPeer } from './hooks/useBeamPeer';
import {
  connectionLabel,
  formatBytes,
  getFriendlyDeviceName,
  normalizePairingCode,
  shortCode,
} from './lib/format';
import type { TransferRecord, TransferUpdate } from './types';

type PairingTab = 'receive' | 'send';

const PUBLIC_APP_URL = 'https://eclipxse.github.io/Eclipxse_beam/';

function fileKey(file: File) {
  return `${file.name}:${file.size}:${file.lastModified}`;
}

export default function App() {
  const nativeCompanion = new URLSearchParams(window.location.search).get('native') === '1';
  const [deviceName, setDeviceName] = useState(() => {
    return localStorage.getItem('beam-device-name') || getFriendlyDeviceName();
  });
  const [activeTab, setActiveTab] = useState<PairingTab>(() => {
    return new URLSearchParams(window.location.search).has('peer') ? 'send' : 'receive';
  });
  const [pairingCode, setPairingCode] = useState(() => {
    return new URLSearchParams(window.location.search).get('peer') ?? '';
  });
  const [selectedFiles, setSelectedFiles] = useState<File[]>([]);
  const [transfers, setTransfers] = useState<TransferRecord[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [copied, setCopied] = useState<'code' | 'link' | null>(null);
  const [localError, setLocalError] = useState('');

  const fileInputRef = useRef<HTMLInputElement>(null);
  const downloadUrlsRef = useRef<string[]>([]);
  const autoConnectAttemptedRef = useRef(false);

  const handleTransferUpdate = useCallback((update: TransferUpdate) => {
    if (update.downloadUrl) downloadUrlsRef.current.push(update.downloadUrl);

    setTransfers((current) => {
      const existingIndex = current.findIndex((item) => item.id === update.id);
      if (existingIndex === -1) return [update, ...current];

      const next = [...current];
      next[existingIndex] = { ...next[existingIndex], ...update };
      return next;
    });
  }, []);

  const {
    peerId,
    status,
    remoteDeviceName,
    error: peerError,
    connectToPeer,
    disconnect,
    sendFiles,
  } = useBeamPeer({ deviceName, onTransferUpdate: handleTransferUpdate });

  useEffect(() => {
    localStorage.setItem('beam-device-name', deviceName);
  }, [deviceName]);

  useEffect(() => {
    if (
      nativeCompanion
      && pairingCode
      && status === 'ready'
      && !autoConnectAttemptedRef.current
    ) {
      autoConnectAttemptedRef.current = true;
      connectToPeer(normalizePairingCode(pairingCode), 'raw');
    }
  }, [connectToPeer, nativeCompanion, pairingCode, status]);

  useEffect(() => {
    return () => {
      downloadUrlsRef.current.forEach((url) => URL.revokeObjectURL(url));
    };
  }, []);

  const shareLink = useMemo(() => {
    if (!peerId) return '';
    const isDesktop = window.location.protocol === 'file:';
    const isProductionWeb = import.meta.env.PROD;
    const url = new URL(isDesktop || isProductionWeb ? PUBLIC_APP_URL : window.location.href);
    url.search = '';
    url.hash = '';
    url.searchParams.set('peer', peerId);
    return url.toString();
  }, [peerId]);

  const selectedSize = useMemo(
    () => selectedFiles.reduce((total, file) => total + file.size, 0),
    [selectedFiles],
  );

  const completedCount = transfers.filter((transfer) => transfer.status === 'complete').length;
  const receivedCount = transfers.filter(
    (transfer) => transfer.direction === 'receiving' && transfer.status === 'complete',
  ).length;

  const addFiles = useCallback((files: File[]) => {
    if (files.length === 0) return;
    setSelectedFiles((current) => {
      const existing = new Set(current.map(fileKey));
      const unique = files.filter((file) => !existing.has(fileKey(file)));
      return [...current, ...unique];
    });
    setLocalError('');
  }, []);

  const copyText = async (value: string, kind: 'code' | 'link') => {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(value);
      setCopied(kind);
      setLocalError('');
      window.setTimeout(() => setCopied(null), 1600);
    } catch {
      setLocalError('Clipboard access was blocked. Select and copy the code manually.');
    }
  };

  const handleConnect = () => {
    const code = normalizePairingCode(pairingCode);
    setPairingCode(code);
    setLocalError('');
    connectToPeer(code, nativeCompanion ? 'raw' : 'default');
  };

  const handleSend = async () => {
    if (selectedFiles.length === 0) {
      setLocalError('Choose at least one file to beam.');
      return;
    }

    setIsSending(true);
    setLocalError('');
    try {
      await sendFiles(selectedFiles);
      setSelectedFiles([]);
    } catch (sendError) {
      setLocalError(sendError instanceof Error ? sendError.message : 'The transfer could not finish.');
    } finally {
      setIsSending(false);
    }
  };

  const statusText = connectionLabel(status);
  const visibleError = localError || peerError;
  const isConnected = status === 'connected';

  return (
    <div className={`app-shell app-shell--${status}`}>
      <div className="desktop-titlebar" aria-hidden="true">
        <span><i /> Eclipxse Beam <em>Desktop</em></span>
      </div>
      <div className="ambient ambient--one" />
      <div className="ambient ambient--two" />
      <div className="beam-grid" aria-hidden="true" />
      <div className="star-field" aria-hidden="true"><i /><i /><i /><i /><i /></div>

      <header className="app-header">
        <BeamLogo />
        <div className="header-actions">
          <div className={`status-chip status-chip--${status}`} aria-live="polite">
            <span className="status-chip__dot" />
            {statusText}
          </div>
          <a
            className="icon-button"
            href="https://github.com/Eclipxse/Eclipxse_beam"
            target="_blank"
            rel="noreferrer"
            aria-label="Open Eclipxse Beam on GitHub"
          >
            <Icon name="github" size={19} />
          </a>
        </div>
      </header>

      <main className="app-main">
        <section className="intro-row">
          <div className="intro-copy">
            <div className="intro-kicker"><span /> Private peer-to-peer transfer</div>
            <h1>Move files. <span>Leave nothing behind.</span></h1>
            <p>Pair two devices and send directly—no account, no upload queue, no cloud copy.</p>
          </div>
          <div className="assurance-row" aria-label="Privacy features">
            <span><Icon name="lock" size={15} /> Encrypted</span>
            <span><Icon name="wifi" size={15} /> Direct</span>
            <span><Icon name="sparkles" size={15} /> Eclipxse</span>
          </div>
        </section>

        <section className="beam-console" aria-label="Eclipxse Beam transfer workspace">
          <header className="console-header">
            <div className="console-heading">
              <span className="console-heading__icon"><Icon name="sparkles" size={17} /></span>
              <div>
                <h2>Transfer workspace</h2>
                <p>Pair a device, choose files, and beam.</p>
              </div>
            </div>
            <label className="device-identity">
              <span>This device</span>
              <input
                value={deviceName}
                onChange={(event) => setDeviceName(event.target.value.slice(0, 32))}
                maxLength={32}
                aria-label="This device name"
              />
            </label>
          </header>

          <div className="console-grid">
            <section className="connection-stage">
              <div className="stage-toolbar">
                <div className="mode-switch" role="tablist" aria-label="Pairing method">
                  <button
                    className={activeTab === 'receive' ? 'is-active' : ''}
                    type="button"
                    role="tab"
                    aria-selected={activeTab === 'receive'}
                    onClick={() => setActiveTab('receive')}
                    disabled={isConnected}
                  >
                    <Icon name="download" size={15} /> Receive
                  </button>
                  <button
                    className={activeTab === 'send' ? 'is-active' : ''}
                    type="button"
                    role="tab"
                    aria-selected={activeTab === 'send'}
                    onClick={() => setActiveTab('send')}
                    disabled={isConnected}
                  >
                    <Icon name="send" size={15} /> Send
                  </button>
                </div>
                <span className="stage-security"><Icon name="lock" size={13} /> WebRTC secured</span>
              </div>

              {isConnected ? (
                <div className="connected-view" role="status">
                  <div className="device-link" aria-hidden="true">
                    <span className="device-link__node"><BeamLogo compact /></span>
                    <span className="device-link__beam"><i /><i /><i /></span>
                    <span className="device-link__node device-link__node--remote"><Icon name="wifi" size={23} /></span>
                  </div>
                  <span className="view-label"><i /> Secure connection active</span>
                  <h3>{remoteDeviceName}</h3>
                  <p>Both devices are ready. Add files to the queue and send whenever you are ready.</p>
                  <button className="disconnect-button" type="button" onClick={disconnect}>Disconnect</button>
                </div>
              ) : activeTab === 'receive' ? (
                <div className="pair-view pair-view--receive" role="tabpanel">
                  <div className="qr-shell">
                    <div className="qr-frame">
                      {shareLink ? (
                        <QRCode value={shareLink} size={188} bgColor="#f8f8fb" fgColor="#10111a" />
                      ) : (
                        <div className="qr-loading"><span /><span /><span /></div>
                      )}
                      <span className="qr-scan-line" />
                    </div>
                    <span className="qr-ready"><i /> Scan with your phone camera</span>
                  </div>

                  <div className="pair-copy">
                    <span className="view-label">Receive on another device</span>
                    <h3>Scan and connect</h3>
                    <p>The QR opens Beam on your phone with this one-time pairing code already filled in.</p>

                    <div className="pairing-code-card">
                      <span>Pairing code</span>
                      <div>
                        <code title={peerId}>{peerId ? shortCode(peerId) : 'Creating code...'}</code>
                        <button
                          className="mini-button"
                          type="button"
                          onClick={() => copyText(peerId, 'code')}
                          disabled={!peerId}
                          aria-label="Copy pairing code"
                        >
                          <Icon name={copied === 'code' ? 'check' : 'copy'} size={16} />
                        </button>
                      </div>
                    </div>

                    <button
                      className="secondary-button"
                      type="button"
                      onClick={() => copyText(shareLink, 'link')}
                      disabled={!shareLink}
                    >
                      <Icon name={copied === 'link' ? 'check' : 'link'} size={16} />
                      {copied === 'link' ? 'Link copied' : 'Copy pairing link'}
                    </button>
                    <span className="public-link-note"><Icon name="lock" size={12} /> Opens the official Eclipxse Beam web app</span>
                  </div>
                </div>
              ) : (
                <div className="pair-view pair-view--send" role="tabpanel">
                  <div className="signal-visual" aria-hidden="true">
                    <span className="signal-visual__core"><Icon name="link" size={25} /></span>
                    <i /><i /><i />
                  </div>
                  <div className="pair-copy">
                    <span className="view-label">Connect to a receiver</span>
                    <h3>Enter their pairing code</h3>
                    <p>Paste a Beam code or a pairing link from the receiving device.</p>
                    <div className="pairing-input-row">
                      <input
                        value={pairingCode}
                        onChange={(event) => setPairingCode(event.target.value)}
                        onKeyDown={(event) => {
                          if (event.key === 'Enter') handleConnect();
                        }}
                        placeholder="Pairing code or link"
                        aria-label="Pairing code or link"
                      />
                      <button type="button" onClick={handleConnect} disabled={status === 'connecting'}>
                        {status === 'connecting' ? 'Connecting...' : 'Connect'}
                        <Icon name="chevron" size={16} />
                      </button>
                    </div>
                    <span className="input-hint"><Icon name="lock" size={12} /> Codes are temporary and private to this session.</span>
                  </div>
                </div>
              )}
            </section>

            <aside className="transfer-dock">
              <div className="dock-heading">
                <div>
                  <span>Send queue</span>
                  <h3>{selectedFiles.length ? `${selectedFiles.length} ready` : 'Choose files'}</h3>
                </div>
                {selectedFiles.length > 0 && <strong>{formatBytes(selectedSize)}</strong>}
              </div>

              <input
                ref={fileInputRef}
                className="visually-hidden"
                type="file"
                multiple
                onChange={(event) => addFiles(Array.from(event.target.files ?? []))}
              />
              <button
                className={`drop-zone ${isDragging ? 'is-dragging' : ''}`}
                type="button"
                onClick={() => fileInputRef.current?.click()}
                onDragEnter={(event) => {
                  event.preventDefault();
                  setIsDragging(true);
                }}
                onDragOver={(event) => event.preventDefault()}
                onDragLeave={(event) => {
                  if (event.currentTarget === event.target) setIsDragging(false);
                }}
                onDrop={(event) => {
                  event.preventDefault();
                  setIsDragging(false);
                  addFiles(Array.from(event.dataTransfer.files));
                }}
              >
                <span className="drop-zone__icon"><Icon name="upload" size={21} /></span>
                <span><strong>{isDragging ? 'Drop to add' : 'Drop files here'}</strong><small>or click to browse</small></span>
                <em>Any type</em>
              </button>

              {selectedFiles.length > 0 && (
                <div className="selected-files" aria-label="Selected files">
                  {selectedFiles.map((file) => (
                    <div className="selected-file" key={fileKey(file)}>
                      <span className="file-icon"><Icon name="file" size={17} /></span>
                      <div>
                        <strong>{file.name}</strong>
                        <span>{formatBytes(file.size)}</span>
                      </div>
                      <button
                        type="button"
                        onClick={() => setSelectedFiles((files) => files.filter((item) => fileKey(item) !== fileKey(file)))}
                        aria-label={`Remove ${file.name}`}
                      >
                        <Icon name="x" size={15} />
                      </button>
                    </div>
                  ))}
                </div>
              )}

              {visibleError && (
                <div className="error-message" role="alert">
                  <Icon name="alert" size={17} />
                  <span>{visibleError}</span>
                  <button type="button" onClick={() => setLocalError('')} aria-label="Dismiss error">
                    <Icon name="x" size={14} />
                  </button>
                </div>
              )}

              <button
                className={`beam-button ${isSending ? 'is-sending' : ''}`}
                type="button"
                onClick={handleSend}
                disabled={!isConnected || selectedFiles.length === 0 || isSending}
              >
                <span>{isSending ? 'Beaming files...' : isConnected ? 'Beam selected files' : 'Connect a device first'}</span>
                <Icon name="send" size={18} />
              </button>

              <div className="activity-dock">
                <div className="activity-heading">
                  <span>Activity</span>
                  <div><strong>{completedCount}</strong> done <i /> <strong>{receivedCount}</strong> received</div>
                </div>

                {transfers.length === 0 ? (
                  <div className="empty-activity">
                    <span><Icon name="sparkles" size={17} /></span>
                    <div><strong>No transfers yet</strong><p>Progress and downloads appear here.</p></div>
                  </div>
                ) : (
                  <div className="transfer-list" aria-live="polite">
                    {transfers.map((transfer) => (
                      <div className="transfer-item" key={transfer.id}>
                        <span className={`transfer-item__icon transfer-item__icon--${transfer.direction}`}>
                          <Icon name={transfer.direction === 'receiving' ? 'download' : 'send'} size={16} />
                        </span>
                        <div className="transfer-item__main">
                          <div className="transfer-item__title">
                            <strong>{transfer.name}</strong>
                            <span>{formatBytes(transfer.size)}</span>
                          </div>
                          <div className="progress-track" aria-label={`${Math.round(transfer.progress)}% complete`}>
                            <span style={{ width: `${transfer.progress}%` }} />
                          </div>
                          <span className={`transfer-state transfer-state--${transfer.status}`}>
                            {transfer.status === 'complete'
                              ? transfer.direction === 'receiving' ? 'Ready to download' : 'Sent successfully'
                              : transfer.status === 'failed' ? transfer.error || 'Transfer failed'
                              : `${transfer.direction === 'receiving' ? 'Receiving' : 'Sending'} · ${Math.round(transfer.progress)}%`}
                          </span>
                        </div>
                        {transfer.downloadUrl && (
                          <a className="download-button" href={transfer.downloadUrl} download={transfer.name} aria-label={`Download ${transfer.name}`}>
                            <Icon name="download" size={16} />
                          </a>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </aside>
          </div>
        </section>

        <section className="trust-row">
          <span><Icon name="lock" size={14} /> Encrypted in transit</span>
          <span><i /> Files never touch our server</span>
          <span><Icon name="github" size={14} /> Open source</span>
        </section>
      </main>

      <footer>
        <BeamLogo compact />
        <p>Private transfer, designed by Eclipxse.</p>
        <span>Desktop v0.3.1</span>
      </footer>
    </div>
  );
}
