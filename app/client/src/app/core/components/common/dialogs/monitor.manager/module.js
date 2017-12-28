"use strict";
var __decorate = (this && this.__decorate) || function (decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for (var i = decorators.length - 1; i >= 0; i--) if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
var __metadata = (this && this.__metadata) || function (k, v) {
    if (typeof Reflect === "object" && typeof Reflect.metadata === "function") return Reflect.metadata(k, v);
};
var core_1 = require("@angular/core");
var common_1 = require("@angular/common");
var component_1 = require("./component");
var component_2 = require("./settings/component");
var component_3 = require("./logs/component");
var components_1 = require("../../../common/components");
var DialogMonitorManagerModule = (function () {
    function DialogMonitorManagerModule() {
    }
    return DialogMonitorManagerModule;
}());
DialogMonitorManagerModule = __decorate([
    core_1.NgModule({
        entryComponents: [component_1.DialogMonitorManager, component_2.DialogMonitorManagerSettingTab, component_3.DialogMonitorManagerLogsTab],
        imports: [common_1.CommonModule, components_1.Components],
        declarations: [component_1.DialogMonitorManager, component_2.DialogMonitorManagerSettingTab, component_3.DialogMonitorManagerLogsTab],
        exports: [component_1.DialogMonitorManager, component_2.DialogMonitorManagerSettingTab, component_3.DialogMonitorManagerLogsTab]
    }),
    __metadata("design:paramtypes", [])
], DialogMonitorManagerModule);
exports.DialogMonitorManagerModule = DialogMonitorManagerModule;
//# sourceMappingURL=module.js.map