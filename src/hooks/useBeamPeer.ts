import { useCallback, useEffect, useRef, useState } from 'react';
import Peer, { type DataConnection } from 'peerjs';
import { getChunkRanges } from '../lib/transfer';
import type {
  BeamMessage,
  ConnectionStatus,
  TransferUpdate,
} from '../types';

const MAX_BUFFERED_BYTES = 4 * 1024 * 1024;

interface PendingFile {
  name: string;
  size: number;
  mimeType: string;
  chunks: ArrayBuffer[];
  receivedBytes: number;
}

interface UseBeamPeerOptions {
  deviceName: string;
  onTransferUpdate: (update: TransferUpdate) => void;
}

const delay = (milliseconds: number) =>
  new Promise<void>((resolve) => window.setTimeout(resolve, milliseconds));

function asArrayBuffer(data: unknown): ArrayBuffer | null {
  if (data instanceof ArrayBuffer) return data;

  if (ArrayBuffer.isView(data)) {
    const start = data.byteOffset;
    const end = start + data.byteLength;
    return data.buffer.slice(start, end) as ArrayBuffer;
  }

  return null;
}

export function useBeamPeer({ deviceName, onTransferUpdate }: UseBeamPeerOptions) {
  const [peerId, setPeerId] = useState('');
  const [status, setStatus] = useState<ConnectionStatus>('starting');
  const [remoteDeviceName, setRemoteDeviceName] = useState('Nearby device');
  const [error, setError] = useState('');

  const peerRef = useRef<Peer | null>(null);
  const connectionRef = useRef<DataConnection | null>(null);
  const pendingFilesRef = useRef(new Map<string, PendingFile>());
  const updateCallbackRef = useRef(onTransferUpdate);
  const deviceNameRef = useRef(deviceName);

  useEffect(() => {
    updateCallbackRef.current = onTransferUpdate;
  }, [onTransferUpdate]);

  useEffect(() => {
    deviceNameRef.current = deviceName;
  }, [deviceName]);

  const closeConnection = useCallback(() => {
    connectionRef.current?.close();
    connectionRef.current = null;
    pendingFilesRef.current.clear();
    setRemoteDeviceName('Nearby device');
    setStatus(peerRef.current?.open ? 'ready' : 'starting');
  }, []);

  const handleMessage = useCallback((payload: unknown) => {
    if (!payload || typeof payload !== 'object' || !('type' in payload)) return;

    const message = payload as BeamMessage;

    if (message.type === 'hello') {
      setRemoteDeviceName(message.deviceName || 'Nearby device');
      return;
    }

    if (message.type === 'file-meta') {
      pendingFilesRef.current.set(message.transferId, {
        name: message.name,
        size: message.size,
        mimeType: message.mimeType,
        chunks: [],
        receivedBytes: 0,
      });
      updateCallbackRef.current({
        id: message.transferId,
        name: message.name,
        size: message.size,
        mimeType: message.mimeType,
        direction: 'receiving',
        status: 'transferring',
        progress: 0,
      });
      return;
    }

    if (message.type === 'file-chunk') {
      const pending = pendingFilesRef.current.get(message.transferId);
      const chunk = asArrayBuffer(message.data);
      if (!pending || !chunk) return;

      pending.chunks.push(chunk);
      pending.receivedBytes += chunk.byteLength;
      updateCallbackRef.current({
        id: message.transferId,
        name: pending.name,
        size: pending.size,
        mimeType: pending.mimeType,
        direction: 'receiving',
        status: 'transferring',
        progress: pending.size === 0 ? 100 : Math.min(100, (pending.receivedBytes / pending.size) * 100),
      });
      return;
    }

    if (message.type === 'file-complete') {
      const pending = pendingFilesRef.current.get(message.transferId);
      if (!pending) return;

      const blob = new Blob(pending.chunks, { type: pending.mimeType || 'application/octet-stream' });
      const downloadUrl = URL.createObjectURL(blob);
      updateCallbackRef.current({
        id: message.transferId,
        name: pending.name,
        size: pending.size,
        mimeType: pending.mimeType,
        direction: 'receiving',
        status: 'complete',
        progress: 100,
        downloadUrl,
      });
      pendingFilesRef.current.delete(message.transferId);
    }
  }, []);

  const configureConnection = useCallback(
    (connection: DataConnection) => {
      connectionRef.current?.close();
      connectionRef.current = connection;
      setStatus('connecting');
      setError('');

      const metadata = connection.metadata as { deviceName?: string } | undefined;
      if (metadata?.deviceName) setRemoteDeviceName(metadata.deviceName);

      connection.on('open', () => {
        setStatus('connected');
        connection.send({ type: 'hello', deviceName: deviceNameRef.current } satisfies BeamMessage);
      });

      connection.on('data', handleMessage);
      connection.on('close', closeConnection);
      connection.on('error', (connectionError) => {
        setError(connectionError.message || 'The connection was interrupted.');
        setStatus('error');
      });
    },
    [closeConnection, handleMessage],
  );

  useEffect(() => {
    const peer = new Peer({ debug: 1 });
    peerRef.current = peer;

    peer.on('open', (id) => {
      setPeerId(id);
      setStatus('ready');
      setError('');
    });
    peer.on('connection', configureConnection);
    peer.on('disconnected', () => {
      if (!peer.destroyed) peer.reconnect();
    });
    peer.on('error', (peerError) => {
      setError(peerError.message || 'Beam could not reach the signaling service.');
      setStatus('error');
    });

    return () => {
      connectionRef.current?.close();
      pendingFilesRef.current.forEach((file) => {
        file.chunks.length = 0;
      });
      pendingFilesRef.current.clear();
      peer.destroy();
      peerRef.current = null;
    };
  }, [configureConnection]);

  const connectToPeer = useCallback(
    (remotePeerId: string) => {
      const peer = peerRef.current;
      const cleanId = remotePeerId.trim();

      if (!peer?.open) {
        setError('Beam is still starting. Try again in a moment.');
        return;
      }
      if (!cleanId || cleanId === peer.id) {
        setError(cleanId === peer.id ? 'That is this device’s own code.' : 'Enter a pairing code first.');
        return;
      }

      const connection = peer.connect(cleanId, {
        reliable: true,
        metadata: { deviceName: deviceNameRef.current },
      });
      configureConnection(connection);
    },
    [configureConnection],
  );

  const waitForBuffer = useCallback(async (connection: DataConnection) => {
    while (
      connection.open &&
      connection.dataChannel &&
      connection.dataChannel.bufferedAmount > MAX_BUFFERED_BYTES
    ) {
      await delay(20);
    }
  }, []);

  const sendFiles = useCallback(
    async (files: File[]) => {
      const connection = connectionRef.current;
      if (!connection?.open) throw new Error('Connect to another device before sending files.');

      for (const file of files) {
        const transferId = crypto.randomUUID();
        const baseUpdate = {
          id: transferId,
          name: file.name,
          size: file.size,
          mimeType: file.type,
          direction: 'sending' as const,
        };

        try {
          updateCallbackRef.current({
            ...baseUpdate,
            status: 'transferring',
            progress: 0,
          });
          connection.send({
            type: 'file-meta',
            transferId,
            name: file.name,
            size: file.size,
            mimeType: file.type,
          } satisfies BeamMessage);

          if (file.size === 0) {
            connection.send({ type: 'file-complete', transferId } satisfies BeamMessage);
          } else {
            for (const range of getChunkRanges(file.size)) {
              if (!connection.open) throw new Error('The other device disconnected.');

              await waitForBuffer(connection);
              const chunk = await file.slice(range.start, range.end).arrayBuffer();
              connection.send({ type: 'file-chunk', transferId, data: chunk } satisfies BeamMessage);
              updateCallbackRef.current({
                ...baseUpdate,
                status: 'transferring',
                progress: Math.min(100, (range.end / file.size) * 100),
              });
            }
            await waitForBuffer(connection);
            connection.send({ type: 'file-complete', transferId } satisfies BeamMessage);
          }

          updateCallbackRef.current({
            ...baseUpdate,
            status: 'complete',
            progress: 100,
          });
        } catch (sendError) {
          updateCallbackRef.current({
            ...baseUpdate,
            status: 'failed',
            progress: 0,
            error: sendError instanceof Error ? sendError.message : 'Transfer failed.',
          });
          throw sendError;
        }
      }
    },
    [waitForBuffer],
  );

  return {
    peerId,
    status,
    remoteDeviceName,
    error,
    connectToPeer,
    disconnect: closeConnection,
    sendFiles,
  };
}
