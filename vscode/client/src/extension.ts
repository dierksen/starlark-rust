/*
 * Copyright 2019 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import { ExtensionContext } from 'vscode';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient';

let client: LanguageClient;

interface AdditionalClientSettings {
    enable_goto_definition: boolean;
}

/// Get a setting at the path, or throw an error if it's not set.
function requireSetting<T>(path: string): T {
    const ret: T = vscode.workspace.getConfiguration().get(path);
    if (ret == undefined) {
        throw new Error(`Setting "${path}" was not configured`)
    }
    return ret;
}

function additionalClientSettings(): AdditionalClientSettings {
    return {
        enable_goto_definition: vscode.workspace.getConfiguration().get("starlark.enableGotoDefinition", true),
    };
}

export function activate(context: ExtensionContext) {
    const path: string = requireSetting("starlark.lspPath");
    const args: [string] = requireSetting("starlark.lspArguments");

    // Otherwise to spawn the server
    let serverOptions: ServerOptions = { command: path, args: args };

    // Options to control the language client
    let clientOptions: LanguageClientOptions = {
        // Register the server for Starlark documents
        documentSelector: [{ scheme: 'file', language: 'starlark' }],
        initializationOptions: additionalClientSettings(),
    };

    // Create the language client and start the client.
    client = new LanguageClient(
        'Starlark',
        'Starlark language server',
        serverOptions,
        clientOptions
    );

    // Start the client. This will also launch the server
    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
