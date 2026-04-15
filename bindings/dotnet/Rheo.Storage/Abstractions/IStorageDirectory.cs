using Rheo.Storage.Models.Information;
using Rheo.Storage.Models.Progress;
using Rheo.Storage.Models.Watching;

namespace Rheo.Storage.Abstractions;

/// <summary>
/// Represents a directory wrapper that exposes enumeration, mutation, and watch capabilities over the native runtime.
/// </summary>
public interface IStorageDirectory : IStorageItem
{
    /// <summary>
    /// Gets cached directory information, refreshing it on first use.
    /// </summary>
    DirectoryInformation Information { get; }

    /// <summary>
    /// Occurs when a watched directory emits a debounced change notification.
    /// </summary>
    event EventHandler<StorageChangedEventArgs>? Changed;

    /// <summary>
    /// Gets a value indicating whether directory watching is active for this instance.
    /// </summary>
    bool IsWatching { get; }

    /// <summary>
    /// Refreshes the cached directory information.
    /// </summary>
    DirectoryInformation RefreshInformation(bool includeSummary = false);

    /// <summary>
    /// Enumerates child files.
    /// </summary>
    IReadOnlyList<StorageFile> GetFiles(bool recursive = false);

    /// <summary>
    /// Enumerates child directories.
    /// </summary>
    IReadOnlyList<StorageDirectory> GetDirectories(bool recursive = false);

    /// <summary>
    /// Enumerates child files and directories.
    /// </summary>
    IReadOnlyList<StorageEntry> GetEntries(bool recursive = false);

    /// <summary>
    /// Resolves a file relative to the current directory.
    /// </summary>
    StorageFile GetFile(string relativePath);

    /// <summary>
    /// Resolves a directory relative to the current directory.
    /// </summary>
    StorageDirectory GetDirectory(string relativePath);

    /// <summary>
    /// Creates the current directory path.
    /// </summary>
    StorageDirectory Create();

    /// <summary>
    /// Creates the current directory path and any missing parents.
    /// </summary>
    StorageDirectory CreateAll();

    /// <summary>
    /// Creates the current directory path asynchronously.
    /// </summary>
    Task<StorageDirectory> CreateAsync(CancellationToken cancellationToken = default);

    /// <summary>
    /// Creates the current directory path and any missing parents asynchronously.
    /// </summary>
    Task<StorageDirectory> CreateAllAsync(CancellationToken cancellationToken = default);

    /// <summary>
    /// Copies the current directory tree to the provided destination path.
    /// </summary>
    IStorageDirectory Copy(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false);

    /// <summary>
    /// Copies the current directory tree to the provided destination path asynchronously.
    /// </summary>
    Task<IStorageDirectory> CopyAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Moves the current directory tree to the provided destination path.
    /// </summary>
    void Move(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false);

    /// <summary>
    /// Moves the current directory tree to the provided destination path asynchronously.
    /// </summary>
    Task MoveAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Renames the current directory within its existing parent directory.
    /// </summary>
    void Rename(string newName);

    /// <summary>
    /// Renames the current directory asynchronously.
    /// </summary>
    Task RenameAsync(string newName, CancellationToken cancellationToken = default);

    /// <summary>
    /// Deletes the current directory.
    /// </summary>
    void Delete(bool recursive = true);

    /// <summary>
    /// Deletes the current directory asynchronously.
    /// </summary>
    Task DeleteAsync(bool recursive = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Starts explicit directory watching for this instance.
    /// </summary>
    void StartWatching(StorageWatchOptions? options = null);

    /// <summary>
    /// Stops directory watching for this instance.
    /// </summary>
    void StopWatching();
}
