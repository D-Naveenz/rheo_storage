using Rheo.Storage.Models.Analysis;
using Rheo.Storage.Models.Information;

namespace Rheo.Storage;

/// <summary>
/// Entry points for creating strongly typed storage wrappers and running direct metadata queries.
/// </summary>
public static class RheoStorage
{
    /// <summary>
    /// Creates a path-based file wrapper.
    /// </summary>
    public static StorageFile File(string path) => new(path);

    /// <summary>
    /// Creates a path-based directory wrapper.
    /// </summary>
    public static StorageDirectory Directory(string path) => new(path);

    /// <summary>
    /// Runs content analysis for a path immediately.
    /// </summary>
    public static AnalysisReport AnalyzePath(string path) => Interop.Native.NativeQueryInvoker.AnalyzePath(path);

    /// <summary>
    /// Queries file information immediately.
    /// </summary>
    public static FileInformation GetFileInformation(string path, bool includeAnalysis = false) =>
        Interop.Native.NativeQueryInvoker.GetFileInformation(path, includeAnalysis);

    /// <summary>
    /// Queries directory information immediately.
    /// </summary>
    public static DirectoryInformation GetDirectoryInformation(string path, bool includeSummary = false) =>
        Interop.Native.NativeQueryInvoker.GetDirectoryInformation(path, includeSummary);
}
