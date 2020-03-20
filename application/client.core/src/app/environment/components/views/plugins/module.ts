import { NgModule                               } from '@angular/core';
import { CommonModule                           } from '@angular/common';
import { ScrollingModule                        } from '@angular/cdk/scrolling';
import { HttpClientModule, HttpClient           } from '@angular/common/http';

import { ViewPluginsComponent                   } from './component';
import { ViewPluginsListComponent               } from './list/component';
import { ViewPluginsPluginComponent             } from './list/plugin/component';
import { ViewPluginsDetailsComponent            } from './details/component';
import { ViewPluginsDetailsLogsComponent        } from './details/logs/component';

import {
    ComplexModule,
    PrimitiveModule,
    ContainersModule                            } from 'chipmunk-client-material';
import { AppDirectiviesModule                   } from '../../../directives/module';

import { MatAutocomplete, MatAutocompleteModule } from '@angular/material/autocomplete';
import { MatButtonModule } from '@angular/material/button';
import { MatOptionModule } from '@angular/material/core';
import { MatFormField, MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatProgressBarModule } from '@angular/material/progress-bar';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatTabsModule } from '@angular/material/tabs';
import { MatExpansionModule } from '@angular/material/expansion';

import {
    FormsModule,
    ReactiveFormsModule } from '@angular/forms';
import { MarkdownModule } from 'ngx-markdown';

const entryComponents = [
    ViewPluginsComponent,
    ViewPluginsListComponent,
    ViewPluginsPluginComponent,
    ViewPluginsDetailsComponent,
    ViewPluginsDetailsLogsComponent,
    MatFormField,
    MatAutocomplete
];
const components = [
    ViewPluginsComponent,
    ViewPluginsListComponent,
    ViewPluginsPluginComponent,
    ViewPluginsDetailsComponent,
    ViewPluginsDetailsLogsComponent
];

@NgModule({
    entryComponents : [ ...entryComponents ],
    imports         : [
        CommonModule,
        ScrollingModule,
        PrimitiveModule,
        ContainersModule,
        ComplexModule,
        FormsModule,
        ReactiveFormsModule,
        MatFormFieldModule,
        MatInputModule,
        MatAutocompleteModule,
        MatOptionModule,
        MatButtonModule,
        AppDirectiviesModule,
        MatProgressSpinnerModule,
        MatProgressBarModule,
        MatTabsModule,
        MatExpansionModule,
        MarkdownModule.forRoot({ loader: HttpClient }),
    ],
    declarations    : [ ...components ],
    exports         : [ ...components ]
})

export class ViewPluginsModule {
    constructor() {
    }
}

