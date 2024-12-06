import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';
import { SwiftPPDebugAdapterDescriptorFactory, SwiftPPConfigurationProvider } from './debugAdapter';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    // Register debug adapter
    const factory = new SwiftPPDebugAdapterDescriptorFactory();
    context.subscriptions.push(vscode.debug.registerDebugAdapterDescriptorFactory('swiftpp', factory));

    // Register debug configuration provider
    const provider = new SwiftPPConfigurationProvider();
    context.subscriptions.push(vscode.debug.registerDebugConfigurationProvider('swiftpp', provider));

    const serverPath = context.asAbsolutePath(
        path.join('out', 'swiftpp-lsp')
    );

    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            transport: TransportKind.stdio,
        },
        debug: {
            command: serverPath,
            transport: TransportKind.stdio,
        }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'swiftpp' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.spp')
        }
    };

    client = new LanguageClient(
        'swiftpp',
        'Swift++ Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
