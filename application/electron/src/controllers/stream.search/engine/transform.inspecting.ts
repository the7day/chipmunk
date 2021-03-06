import * as Stream from 'stream';
import Logger from '../../../tools/env.logger';

export type TMap = { [key: number]: string[] };
export type TStats = { [key: string]: number };

export default class Transform extends Stream.Transform {

    private _logger: Logger;
    private _rest: string = '';
    private _lines: number[] = [];
    private _stopped: boolean = false;

    constructor(options: Stream.TransformOptions) {
        super(options);
        this._logger = new Logger(`Transform.Inspecting`);
    }

    public getLines(): number[] {
        return this._lines;
    }

    public _transform(chunk: Buffer | string, encoding: string, callback: Stream.TransformCallback | undefined): void {
        // Convert to utf8 and insert rest from previos
        let output: string = '';
        if (typeof chunk === 'string') {
            output = this._rest + chunk;
        } else {
            output = this._rest + chunk.toString('utf8');
        }
        // Get rest from the end
        const rest = this._getRest(output);
        this._rest = rest.rest;
        output = rest.cleared;
        // Get indexes
        const matches: RegExpMatchArray | null = output.match(/\u0002\d*\u0002/gi);
        if (matches !== null) {
            matches.forEach((match: string) => {
                const num: number = parseInt(match.replace(/\u0002/gi, ''), 10);
                if (isNaN(num) || !isFinite(num)) {
                    this._logger.warn(`Fail to detect line number for ${match}`);
                    return;
                }
                this._lines.push(num);
            });
        }
        // Check state
        if (this._stopped) {
            return;
        }
        // Call callback
        if (callback !== undefined) {
            callback(undefined, output);
        }
    }

    public stop() {
        this._stopped = true;
    }

    private _getRest(str: string): { rest: string, cleared: string } {
        const last = str.length - 1;
        for (let i = last; i >= 0; i -= 1) {
            if (str[i] === '\n' && i > 0) {
                return {
                    rest: str.substr(i + 1, last),
                    cleared: str.substr(0, i + 1),
                };
            }
        }
        return { rest: '', cleared: str };
    }

}
