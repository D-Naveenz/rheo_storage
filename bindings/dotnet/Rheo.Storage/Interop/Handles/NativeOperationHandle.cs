using Rheo.Storage.Interop.Native;
using Rheo.Storage.Models.Progress;
using Microsoft.Extensions.Logging;
using Rheo.Storage.Core;

namespace Rheo.Storage.Interop.Handles;

internal sealed class NativeOperationHandle : IDisposable
{
    private nint _handle;

    internal NativeOperationHandle(nint handle)
    {
        _handle = handle;
    }

    internal static NativeOperationHandle Create(Func<(NativeStatus Status, nint Handle, nint ErrorPtr, nuint ErrorLen)> starter)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var (status, handle, errorPtr, errorLen) = starter();
        return Create(status, handle, errorPtr, errorLen);
    }

    internal static NativeOperationHandle Create(NativeStatus status, nint handle, nint errorPtr, nuint errorLen)
    {
        NativeHelpers.EnsureSupportedPlatform();
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return new NativeOperationHandle(handle);
    }

    internal NativeOperationSnapshot GetSnapshot()
    {
        ThrowIfDisposed();
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeOperations.rheo_operation_get_snapshot(_handle, out var snapshot, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return snapshot;
    }

    internal void Cancel()
    {
        if (_handle == 0)
        {
            return;
        }

        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeOperations.rheo_operation_cancel(_handle, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
    }

    internal string TakeStringResult()
    {
        ThrowIfDisposed();
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeOperations.rheo_operation_take_string_result(_handle, out var valuePtr, out var valueLen, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return NativeMemory.ReadUtf8AndFree(valuePtr, valueLen);
    }

    internal byte[] TakeBytesResult()
    {
        ThrowIfDisposed();
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeOperations.rheo_operation_take_bytes_result(_handle, out var valuePtr, out var valueLen, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return NativeMemory.ReadBytesAndFree(valuePtr, valueLen);
    }

    internal NativeErrorPayload? GetError()
    {
        ThrowIfDisposed();
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeOperations.rheo_operation_get_error(_handle, out var jsonPtr, out var jsonLen, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        var json = NativeMemory.ReadUtf8AndFree(jsonPtr, jsonLen);
        return string.IsNullOrWhiteSpace(json) || string.Equals(json, "null", StringComparison.OrdinalIgnoreCase)
            ? null
            : NativeJson.Deserialize<NativeErrorPayload>(json);
    }

    internal async Task<StorageProgress> WaitForCompletionAsync(
        IProgress<StorageProgress>? progress,
        CancellationToken cancellationToken)
    {
        RheoStorageLogBridge.LogManaged(
            LogLevel.Debug,
            "Rheo.Storage.NativeOperationHandle",
            "Waiting for native storage operation to complete.");
        using var registration = cancellationToken.Register(static state => ((NativeOperationHandle)state!).Cancel(), this);

        while (true)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var snapshot = GetSnapshot();
            var model = snapshot.ToModel();
            progress?.Report(model);

            switch (snapshot.State)
            {
                case NativeOperationState.Running:
                    await Task.Delay(75, CancellationToken.None).ConfigureAwait(false);
                    continue;
                case NativeOperationState.Completed:
                    RheoStorageLogBridge.LogManaged(
                        LogLevel.Information,
                        "Rheo.Storage.NativeOperationHandle",
                        "Native storage operation completed successfully.");
                    return model;
                case NativeOperationState.Cancelled:
                    RheoStorageLogBridge.LogManaged(
                        LogLevel.Warning,
                        "Rheo.Storage.NativeOperationHandle",
                        "Native storage operation was cancelled.");
                    throw new OperationCanceledException("Native storage operation was cancelled.", cancellationToken);
                case NativeOperationState.Failed:
                    var error = GetError();
                    RheoStorageLogBridge.LogManaged(
                        LogLevel.Error,
                        "Rheo.Storage.NativeOperationHandle",
                        error?.Message ?? "Native storage operation failed without an error payload.");
                    throw error is null
                        ? new InvalidOperationException("Native storage operation failed without an error payload.")
                        : new Exceptions.RheoStorageException(error.Message, error.Code, error.Path, error.Operation);
                default:
                    throw new InvalidOperationException($"Unknown native operation state '{snapshot.State}'.");
            }
        }
    }

    public void Dispose()
    {
        if (_handle == 0)
        {
            return;
        }

        NativeHelpers.EnsureSupportedPlatform();
        NativeOperations.rheo_operation_free(_handle);
        _handle = 0;
        GC.SuppressFinalize(this);
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_handle == 0, this);
    }
}
