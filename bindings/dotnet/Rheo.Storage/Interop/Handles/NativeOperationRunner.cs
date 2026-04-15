using Rheo.Storage.Models.Progress;

namespace Rheo.Storage.Interop.Handles;

internal static class NativeOperationRunner
{
    internal static async Task<TResult> RunAsync<TResult>(
        NativeOperationHandle handle,
        Func<NativeOperationHandle, TResult> resultSelector,
        IProgress<StorageProgress>? progress,
        CancellationToken cancellationToken)
    {
        using (handle)
        {
            await handle.WaitForCompletionAsync(progress, cancellationToken).ConfigureAwait(false);
            return resultSelector(handle);
        }
    }
}
