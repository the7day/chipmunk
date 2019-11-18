// tslint:disable:no-inferrable-types

import { Component, ChangeDetectorRef, Input, OnInit, AfterViewInit, Output, OnDestroy } from '@angular/core';
import { IPortInfo, IPortState } from '../../common/interface.portinfo';
import { IOptions, CDefaultOptions} from '../../common/interface.options';

import * as Toolkit from 'chipmunk.client.toolkit';

interface IConnected {
    port: IPortInfo;
    options: IOptions;
    state: IPortState;
}

@Component({
    selector: 'lib-sb-port-dialog-com',
    templateUrl: './template.html',
    styleUrls: ['./styles.less']
})

export class SidebarVerticalPortDialogComponent implements OnInit, OnDestroy {

    @Input() private _onConnect: () => void;
    @Input() private _requestPortList: () => IPortInfo[];
    @Input() private _getSelected: (IPortInfo) => void;

    @Input() public _ng_getState: (IPortInfo) => IPortState
    @Input() public _ng_canBeConnected: () => boolean;
    @Input() public _ng_connected: IConnected[];
    @Input() public _ng_isPortSelected: (port: IPortInfo) => boolean;
    @Input() public _ng_onOptions: () => void;
    @Input() public _ng_onPortSelect: (port: IPortInfo) => void;
    @Input() public _ng_getSpyState: () => { [key: string]: number };

    private interval: any;

    public _ng_ports: IPortInfo[] = [];
    public _ng_selected: IPortInfo | undefined;
    public _ng_busy: boolean = false;
    public _ng_error: string | undefined;
    public _ng_options: boolean = false;

    constructor(private _cdRef: ChangeDetectorRef) {
    }
    
    ngOnInit() {
        this._ng_ports = this._requestPortList();
        this.interval = setInterval(() => {
            this._cdRef.detectChanges();
        }, 200);
    }

    ngOnDestroy() {
        clearInterval(this.interval);
    }

    public _ng_onConnect() {
        this._getSelected(this._ng_selected);
        this._onConnect();
    }
}