import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import QRCode from 'react-qr-code';
import { BeamLogo } from './components/BeamLogo';
import { CelestialSeal } from './components/CelestialSeal';
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

function fileKey(file: File) {
  return `${file.name}:${file.size}:${file.lastModified}`;
}

export default function App() {
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
    return () => {
      downloadUrlsRef.current.forEach((url) => URL.revokeObjectURL(url));
    };
  }, []);

  const shareLink = useMemo(() => {
    if (!peerId) return '';
    const url = new URL(
      window.location.protocol === 'file:'
        ? 'https://eclipxse.github.io/Eclipxse_beam/'
        : window.location.href,
    );
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
      setLocalError('Your browser blocked clipboard access. Select and copy the code manually.');
    }
  };

  const handleConnect = () => {
    const code = normalizePairingCode(pairingCode);
    setPairingCode(code);
    setLocalError('');
    connectToPeer(code);
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

  return (
    <div className="app-shell">
      <div className="desktop-titlebar" aria-hidden="true">
        <span><i /> Eclipxse Beam <em>Desktop</em></span>
      </div>
      <div className="ambient ambient--one" />
      <div className="ambient ambient--two" />
      <div className="stars" aria-hidden="true" />
      <div className="cathedral-lines" aria-hidden="true" />

      <header className="site-header">
        <BeamLogo />
        <div className="header-actions">
          <span className="privacy-pill">
            <Icon name="lock" size={15} />
            The veil is encrypted
          </span>
          <a
            className="icon-button"
            href="https://github.com/Eclipxse/Eclipxse_beam"
            target="_blank"
            rel="noreferrer"
            aria-label="Open Eclipxse Beam on GitHub"
          >
            <Icon name="github" size={21} />
          </a>
        </div>
      </header>

      <main>
        <section className="hero">
          <CelestialSeal />
          <div className="hero-copy">
            <div className="eyebrow">
              <span className="eyebrow-star">✦</span>
              A private passage between worlds
              <span className="eyebrow-star">✦</span>
            </div>
            <h1>
              Send through<br />the <span>veil.</span>
            </h1>
            <p>
              A sacred little doorway for your files. No account, no cloud vault,
              no stranger keeping what belongs to you.
            </p>
            <div className="hero-points" aria-label="Beam benefits">
              <span><Icon name="lock" size={16} /> Encrypted</span>
              <span><Icon name="wifi" size={16} /> Soul to soul</span>
              <span><Icon name="sparkles" size={16} /> Eclipxse made</span>
            </div>
          </div>
        </section>

        <section className="beam-workspace" aria-label="File transfer workspace">
          <article className="panel pairing-panel">
            <div className="panel-heading">
              <div>
                <span className="step-number">I</span>
                <h2>Open the passage</h2>
              </div>
              <div className={`status-badge status-badge--${status}`}>
                <span className="status-badge__dot" />
                {statusText}
              </div>
            </div>

            <label className="device-name-field">
              <span>This device</span>
              <input
                value={deviceName}
                onChange={(event) => setDeviceName(event.target.value.slice(0, 32))}
                maxLength={32}
                aria-label="This device name"
              />
            </label>

            {status === 'connected' ? (
              <div className="connected-state">
                <div className="connection-orbit" aria-hidden="true">
                  <div className="device-node"><BeamLogo compact /></div>
                  <div className="orbit-line"><i /><i /><i /></div>
                  <div className="device-node device-node--remote"><Icon name="wifi" size={24} /></div>
                </div>
                <div>
                  <span className="overline">Secure tunnel open</span>
                  <h3>{remoteDeviceName}</h3>
                  <p>Both devices are ready. Choose something wonderful to send.</p>
                </div>
                <button className="text-button text-button--danger" type="button" onClick={disconnect}>
                  Disconnect
                </button>
              </div>
            ) : (
              <>
                <div className="tabs" role="tablist" aria-label="Pairing method">
                  <button
                    className={activeTab === 'receive' ? 'is-active' : ''}
                    type="button"
                    role="tab"
                    aria-selected={activeTab === 'receive'}
                    onClick={() => setActiveTab('receive')}
                  >
                    Receive
                  </button>
                  <button
                    className={activeTab === 'send' ? 'is-active' : ''}
                    type="button"
                    role="tab"
                    aria-selected={activeTab === 'send'}
                    onClick={() => setActiveTab('send')}
                  >
                    Send
                  </button>
                </div>

                {activeTab === 'receive' ? (
                  <div className="receive-view" role="tabpanel">
                    <div className="qr-frame">
                      {shareLink ? (
                        <QRCode value={shareLink} size={148} bgColor="transparent" fgColor="#19162d" />
                      ) : (
                        <div className="qr-loading"><span /><span /><span /></div>
                      )}
                    </div>
                    <div className="pairing-details">
                      <span className="overline">Your celestial cipher</span>
                      <div className="code-row">
                        <code title={peerId}>{peerId ? shortCode(peerId) : 'creating-code'}</code>
                        <button
                          className="mini-button"
                          type="button"
                          onClick={() => copyText(peerId, 'code')}
                          disabled={!peerId}
                          aria-label="Copy pairing code"
                        >
                          <Icon name={copied === 'code' ? 'check' : 'copy'} size={17} />
                        </button>
                      </div>
                      <p>Let the other device scan this seal, or share the private passage.</p>
                      <button
                        className="secondary-button"
                        type="button"
                        onClick={() => copyText(shareLink, 'link')}
                        disabled={!shareLink}
                      >
                        <Icon name={copied === 'link' ? 'check' : 'link'} size={17} />
                        {copied === 'link' ? 'Passage copied' : 'Copy private passage'}
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="send-view" role="tabpanel">
                    <div className="send-illustration" aria-hidden="true">
                      <Icon name="link" size={28} />
                    </div>
                    <div>
                      <span className="overline">Cross the threshold</span>
                      <h3>Enter their Beam cipher</h3>
                      <p>A private passage link works here too.</p>
                    </div>
                    <div className="pairing-input-row">
                      <input
                        value={pairingCode}
                        onChange={(event) => setPairingCode(event.target.value)}
                        onKeyDown={(event) => {
                          if (event.key === 'Enter') handleConnect();
                        }}
                        placeholder="Pairing cipher or private link"
                        aria-label="Pairing code or link"
                      />
                      <button type="button" onClick={handleConnect} disabled={status === 'connecting'}>
                        {status === 'connecting' ? 'Connecting…' : 'Connect'}
                        <Icon name="chevron" size={17} />
                      </button>
                    </div>
                  </div>
                )}
              </>
            )}
          </article>

          <article className="panel transfer-panel">
            <div className="panel-heading">
              <div>
                <span className="step-number">II</span>
                <h2>Choose your offering</h2>
              </div>
              {selectedFiles.length > 0 && (
                <span className="selection-summary">
                  {selectedFiles.length} {selectedFiles.length === 1 ? 'file' : 'files'} · {formatBytes(selectedSize)}
                </span>
              )}
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
              <span className="drop-zone__icon"><Icon name="upload" size={27} /></span>
              <strong>{isDragging ? 'Release them into the veil' : 'Lay your files upon the altar'}</strong>
              <span>or click to browse · every file is welcome</span>
            </button>

            {selectedFiles.length > 0 && (
              <div className="selected-files">
                {selectedFiles.map((file) => (
                  <div className="selected-file" key={fileKey(file)}>
                    <span className="file-icon"><Icon name="file" size={19} /></span>
                    <div>
                      <strong>{file.name}</strong>
                      <span>{formatBytes(file.size)}</span>
                    </div>
                    <button
                      type="button"
                      onClick={() => setSelectedFiles((files) => files.filter((item) => fileKey(item) !== fileKey(file)))}
                      aria-label={`Remove ${file.name}`}
                    >
                      <Icon name="x" size={16} />
                    </button>
                  </div>
                ))}
              </div>
            )}

            {visibleError && (
              <div className="error-message" role="alert">
                <Icon name="alert" size={18} />
                <span>{visibleError}</span>
                <button type="button" onClick={() => setLocalError('')} aria-label="Dismiss error">
                  <Icon name="x" size={15} />
                </button>
              </div>
            )}

            <button
              className="beam-button"
              type="button"
              onClick={handleSend}
              disabled={status !== 'connected' || selectedFiles.length === 0 || isSending}
            >
              <span>{isSending ? 'Crossing the veil...' : status === 'connected' ? 'Send through the veil' : 'Open a passage to continue'}</span>
              <Icon name="send" size={20} />
            </button>
          </article>
        </section>

        <section className="activity-section">
          <div className="activity-heading">
            <div>
              <span className="overline">The living archive</span>
              <h2>Passages made</h2>
            </div>
            <div className="activity-stats">
              <span><strong>{completedCount}</strong> completed</span>
              <span><strong>{receivedCount}</strong> received</span>
            </div>
          </div>

          {transfers.length === 0 ? (
            <div className="empty-activity">
              <span><Icon name="sparkles" size={22} /></span>
              <div>
                <strong>The chapel is quiet</strong>
                <p>Your active and completed passages will gather here.</p>
              </div>
            </div>
          ) : (
            <div className="transfer-list" aria-live="polite">
              {transfers.map((transfer) => (
                <div className="transfer-item" key={transfer.id}>
                  <span className={`transfer-item__icon transfer-item__icon--${transfer.direction}`}>
                    <Icon name={transfer.direction === 'receiving' ? 'download' : 'send'} size={19} />
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
                        ? transfer.direction === 'receiving' ? 'Ready to download' : 'Beamed successfully'
                        : transfer.status === 'failed' ? transfer.error || 'Transfer failed'
                        : `${transfer.direction === 'receiving' ? 'Receiving' : 'Sending'} · ${Math.round(transfer.progress)}%`}
                    </span>
                  </div>
                  {transfer.downloadUrl && (
                    <a className="download-button" href={transfer.downloadUrl} download={transfer.name}>
                      <Icon name="download" size={18} />
                      Download
                    </a>
                  )}
                </div>
              ))}
            </div>
          )}
        </section>

        <section className="trust-strip">
          <div><Icon name="lock" size={20} /><span><strong>Sealed in transit</strong>WebRTC guards every byte.</span></div>
          <div><Icon name="wifi" size={20} /><span><strong>No earthly vault</strong>Files never rest on our server.</span></div>
          <div><Icon name="github" size={20} /><span><strong>Born open</strong>Transparent code, signed Eclipxse.</span></div>
        </section>
      </main>

      <footer>
        <BeamLogo compact />
        <p>What is yours passes only through your hands.</p>
        <span>Made by Eclipxse · desktop v0.2.0</span>
      </footer>
    </div>
  );
}
