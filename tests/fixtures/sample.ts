import { EventEmitter } from 'events';

export interface Config {
    port: number;
    host: string;
}

export class Server {
    private config: Config;
    private emitter: EventEmitter;

    constructor(config: Config) {
        this.config = config;
        this.emitter = new EventEmitter();
    }

    start(): void {
        this.emitter.emit('start');
        console.log(`Server started on ${this.config.host}:${this.config.port}`);
    }

    stop(): void {
        this.emitter.emit('stop');
    }
}

export function createServer(port: number): Server {
    return new Server({ port, host: 'localhost' });
}

export const DEFAULT_PORT: number = 3000;

export type Handler = (req: Request, res: Response) => void;
