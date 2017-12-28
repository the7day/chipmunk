"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var core_1 = require("@angular/core");
var ServiceTopBarMenuItems = (function () {
    function ServiceTopBarMenuItems() {
    }
    ServiceTopBarMenuItems.prototype.getItems = function () {
        return [
            { icon: 'fa-upload', caption: 'Open log file', handle: 'openLocalFile', type: 'item' },
            { icon: 'fa-plug', caption: 'Open stream from serial', handle: 'openSerialStream', type: 'item' },
            { icon: 'fa-android', caption: 'Open stream from ADB logcat', handle: 'openADBLogcatStream', type: 'item' },
            { icon: 'fa-terminal', caption: 'Terminal command', handle: 'openTerminalCommand', type: 'item' },
            { icon: 'fa-stethoscope', caption: 'Monitor manager', handle: 'openMonitorManager', type: 'item' },
            { type: 'line' },
            { icon: 'fa-desktop', caption: 'Add view', handle: 'addView', type: 'item' },
            { type: 'line' },
            { icon: 'fa-cloud', caption: 'Connect to service', handle: 'connectionSettings', type: 'item' },
            { type: 'line' },
            { icon: 'fa-paint-brush', caption: 'Change color theme', handle: 'changeThemeSettings', type: 'item' },
            { type: 'line' },
            { icon: 'fa-child', caption: 'About', handle: null, type: 'item' },
        ];
    };
    return ServiceTopBarMenuItems;
}());
ServiceTopBarMenuItems = __decorate([
    core_1.Injectable()
], ServiceTopBarMenuItems);
exports.ServiceTopBarMenuItems = ServiceTopBarMenuItems;
//# sourceMappingURL=service.topbar.menu.js.map