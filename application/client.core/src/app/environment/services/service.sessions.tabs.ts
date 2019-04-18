import { TabsService, DockingComponent, DockDef, DocksService } from 'logviewer-client-complex';
import { Subscription } from './service.electron.ipc';
import { ControllerSessionTab } from '../controller/controller.session.tab';
import * as Toolkit from 'logviewer.client.toolkit';
import { IService } from '../interfaces/interface.service';
import { Observable, Subject } from 'rxjs';
import { ViewOutputComponent } from '../components/views/output/component';
import ElectronIpcService, { IPCMessages } from './service.electron.ipc';

type TSessionGuid = string;

export class TabsSessionsService implements IService {

    private _logger: Toolkit.Logger = new Toolkit.Logger('TabsSessionsService');
    private _sessions: Map<TSessionGuid, ControllerSessionTab> = new Map();
    private _tabsService: TabsService = new TabsService();
    private _subscriptions: { [key: string]: Subscription | undefined } = { };
    private _currentSessionGuid: string;
    private _subjects: {
        onSessionChange: Subject<ControllerSessionTab>
    } = {
        onSessionChange: new Subject<ControllerSessionTab>()
    };
    constructor() {

    }

    public init(): Promise<void> {
        return new Promise((resolve, reject) => {
            resolve();
        });
    }

    public getName(): string {
        return 'TabsSessionsService';
    }

    public destroy() {
        Object.keys(this._subscriptions).forEach((key: string) => {
            this._subscriptions[key].destroy();
        });
    }

    public create(): void {
        const guid: string = Toolkit.guid();
        const session = new ControllerSessionTab({
            guid: guid,
            transports: ['file', 'processes', 'dlt'],
        });
        this._tabsService.add({
            guid: guid,
            name: 'Default',
            active: true,
            content: {
                factory: DockingComponent,
                inputs: {
                    service: new DocksService(guid, new DockDef.Container({
                        a: new DockDef.Dock({ caption: 'Default', component: {
                            factory: ViewOutputComponent,
                            inputs: {
                                session: session
                            }
                        } })
                    }))
                }
            }
        });
        this._sessions.set(guid, session);
        this.setActive(guid);
    }

    public getTabsService(): TabsService {
        return this._tabsService;
    }

    public getObservable(): {
        onSessionChange: Observable<ControllerSessionTab>
    } {
        return {
            onSessionChange: this._subjects.onSessionChange.asObservable()
        };
    }

    public setActive(guid: string) {
        const session: ControllerSessionTab | undefined = this._sessions.get(guid);
        if (session === undefined) {
            return this._logger.warn(`Cannot fild session ${guid}. Cannot make this session active.`);
        }
        this._currentSessionGuid = guid;
        this._tabsService.setActive(this._currentSessionGuid);
        this._subjects.onSessionChange.next(session);
        ElectronIpcService.send(new IPCMessages.StreamSetActive({ guid: this._currentSessionGuid })).catch((error: Error) => {
            this._logger.warn(`Fail to send notification about active session due error: ${error.message}`);
        });
    }

    public getActive(): ControllerSessionTab | undefined {
        return this._sessions.get(this._currentSessionGuid);
    }

}

export default (new TabsSessionsService());


        /*
        this.tabsService.add({
            name: 'Tab 2 (2)',
            active: false,
            content: {
                factory: DockingComponent,
                inputs: {
                    service: new DocksService('1', new DockDef.Container({
                        a: new DockDef.Dock({ caption: 'Dock 1' }),
                        b: new DockDef.Dock({ caption: 'Dock 2' })
                    }))
                }
            }
        });
        this.tabsService.add({
            name: 'Tab 3 (4)',
            active: false,
            content: {
                factory: DockingComponent,
                inputs: {
                    service: new DocksService('1', new DockDef.Container({
                        a: new DockDef.Container({
                            a: new DockDef.Dock({ caption: '1' }),
                            b: new DockDef.Dock({ caption: '2' })
                        }),
                        b: new DockDef.Container({
                            a: new DockDef.Dock({ caption: '3' }),
                            b: new DockDef.Dock({ caption: '4' })
                        })
                    }))
                }
            }
        });
        this.tabsService.add({
            name: 'Tab 4 (5)',
            active: false,
            content: {
                factory: DockingComponent,
                inputs: {
                    service: new DocksService('1', new DockDef.Container({
                        a: new DockDef.Container({
                            a: new DockDef.Dock({ caption: 'Dock 1' }),
                            b: new DockDef.Dock({ caption: 'Dock 2' })
                        }),
                        b: new DockDef.Container({
                            a: new DockDef.Dock({ caption: 'Dock 3' }),
                            b: new DockDef.Container({
                                a: new DockDef.Dock({ caption: 'Dock 4' }),
                                b: new DockDef.Dock({ caption: 'Dock 5' })
                            })
                        })
                    }))
                }
            }
        });
        this.tabsService.add({
            name: 'Tab 5',
            active: false,
            content: {
                factory: DockingComponent,
                inputs: {
                    service: new DocksService('1', new DockDef.Container({
                        a: new DockDef.Dock({ caption: 'Dock 1' })
                    }))
                }
            }
        });
        */
