import * as vscode from 'vscode';
import { WorkspaceFolder, DebugConfiguration, ProviderResult, CancellationToken } from 'vscode';

export class SwiftPPDebugAdapterDescriptorFactory implements vscode.DebugAdapterDescriptorFactory {
    createDebugAdapterDescriptor(
        session: vscode.DebugSession,
        executable: vscode.DebugAdapterExecutable | undefined
    ): ProviderResult<vscode.DebugAdapterDescriptor> {
        // Get the path to our debug adapter
        const adapterPath = vscode.workspace
            .getConfiguration('swiftpp')
            .get<string>('debugAdapterPath', 'swiftpp-debug-adapter');

        return new vscode.DebugAdapterExecutable(
            adapterPath,
            []
        );
    }
}

export class SwiftPPConfigurationProvider implements vscode.DebugConfigurationProvider {
    resolveDebugConfiguration(
        folder: WorkspaceFolder | undefined,
        config: DebugConfiguration,
        token?: CancellationToken
    ): ProviderResult<DebugConfiguration> {
        // If launch.json is missing or empty
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'swiftpp') {
                config.type = 'swiftpp';
                config.name = 'Launch';
                config.request = 'launch';
                config.program = '${file}';
                config.stopOnEntry = true;
            }
        }

        if (!config.program) {
            return vscode.window.showInformationMessage('Cannot find a program to debug').then(_ => {
                return undefined; // abort launch
            });
        }

        return config;
    }
}
