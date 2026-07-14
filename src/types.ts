export type ConnectionStatus =
  | 'starting'
  | 'ready'
  | 'connecting'
  | 'connected'
  | 'error';

export type TransferDirection = 'sending' | 'receiving';

export type TransferStatus = 'waiting' | 'transferring' | 'complete' | 'failed';

export interface TransferRecord {
  id: string;
  name: string;
  size: number;
  mimeType: string;
  direction: TransferDirection;
  status: TransferStatus;
  progress: number;
  downloadUrl?: string;
  error?: string;
}

export interface TransferUpdate {
  id: string;
  name: string;
  size: number;
  mimeType: string;
  direction: TransferDirection;
  status: TransferStatus;
  progress: number;
  downloadUrl?: string;
  error?: string;
}

export type BeamMessage =
  | {
      type: 'hello';
      deviceName: string;
    }
  | {
      type: 'file-meta';
      transferId: string;
      name: string;
      size: number;
      mimeType: string;
    }
  | {
      type: 'file-chunk';
      transferId: string;
      data: ArrayBuffer;
    }
  | {
      type: 'file-complete';
      transferId: string;
    };
