using Rheo.Storage.Models.Analysis;
using Rheo.Storage.Models.Information;
using Rheo.Storage.Models.Progress;

namespace Rheo.Storage.Abstractions;

/// <summary>
/// Represents a file wrapper that exposes sync and async operations over the native Rheo Storage runtime.
/// </summary>
public interface IStorageFile : IStorageItem
{
    /// <summary>
    /// Gets cached file information, refreshing it on first use.
    /// </summary>
    FileInformation Information { get; }

    /// <summary>
    /// Refreshes the cached file information.
    /// </summary>
    FileInformation RefreshInformation(bool includeAnalysis = false);

    /// <summary>
    /// Runs content analysis for the current file.
    /// </summary>
    AnalysisReport Analyze();

    /// <summary>
    /// Reads the current file into memory as raw bytes.
    /// </summary>
    byte[] ReadBytes();

    /// <summary>
    /// Reads the current file into memory asynchronously, optionally reporting progress.
    /// </summary>
    Task<byte[]> ReadBytesAsync(IProgress<StorageProgress>? progress = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// Reads the current file as UTF-8 text.
    /// </summary>
    string ReadText();

    /// <summary>
    /// Reads the current file as UTF-8 text asynchronously, optionally reporting progress.
    /// </summary>
    Task<string> ReadTextAsync(IProgress<StorageProgress>? progress = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// Writes raw bytes to the current file.
    /// </summary>
    void Write(byte[] content, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true);

    /// <summary>
    /// Writes raw bytes to the current file asynchronously.
    /// </summary>
    Task WriteAsync(byte[] content, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Writes UTF-8 text to the current file.
    /// </summary>
    void WriteText(string text, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true);

    /// <summary>
    /// Writes UTF-8 text to the current file asynchronously.
    /// </summary>
    Task WriteTextAsync(string text, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Streams content into the current file asynchronously.
    /// </summary>
    Task WriteAsync(Stream stream, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Copies the current file to the provided destination path.
    /// </summary>
    IStorageFile Copy(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false);

    /// <summary>
    /// Copies the current file to the provided destination path asynchronously.
    /// </summary>
    Task<IStorageFile> CopyAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Moves the current file to the provided destination path.
    /// </summary>
    void Move(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false);

    /// <summary>
    /// Moves the current file to the provided destination path asynchronously.
    /// </summary>
    Task MoveAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Renames the current file within its existing parent directory.
    /// </summary>
    void Rename(string newName);

    /// <summary>
    /// Renames the current file asynchronously.
    /// </summary>
    Task RenameAsync(string newName, CancellationToken cancellationToken = default);

    /// <summary>
    /// Deletes the current file.
    /// </summary>
    void Delete();

    /// <summary>
    /// Deletes the current file asynchronously.
    /// </summary>
    Task DeleteAsync(CancellationToken cancellationToken = default);
}
