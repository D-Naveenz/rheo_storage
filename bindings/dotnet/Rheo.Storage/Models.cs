using System.Text.Json;
using System.Text.Json.Serialization;

namespace Rheo.Storage;

public sealed class RheoStorageException : Exception
{
    public RheoStorageException(string message, string code, string? path = null, string? operation = null)
        : base(message)
    {
        Code = code;
        PathValue = path;
        Operation = operation;
    }

    public string Code { get; }

    public string? PathValue { get; }

    public string? Operation { get; }
}

internal sealed record NativeError(
    string Code,
    string Message,
    string? Path,
    string? Operation,
    string? Kind,
    string? Value);

public sealed record StorageMetadata(
    string Path,
    string Name,
    bool IsReadOnly,
    bool IsHidden,
    bool IsSystem,
    bool IsTemporary,
    bool IsSymbolicLink,
    string? LinkTarget,
    long? CreatedAtUtcMs,
    long? ModifiedAtUtcMs,
    long? AccessedAtUtcMs);

public sealed record DetectedDefinition(
    string FileTypeLabel,
    string MimeType,
    IReadOnlyList<string> Extensions,
    ulong Score,
    double Confidence);

public sealed record AnalysisReport(
    IReadOnlyList<DetectedDefinition> Matches,
    string? TopMimeType,
    string? TopDetectedExtension,
    string ContentKind,
    int BytesScanned,
    ulong FileSize,
    string? SourceExtension);

public sealed record FileInfo(
    StorageMetadata Metadata,
    string DisplayName,
    ulong Size,
    string FormattedSize,
    string? FilenameExtension,
    AnalysisReport? Analysis);

public sealed record DirectorySummary(
    ulong TotalSize,
    ulong FileCount,
    ulong DirectoryCount,
    string FormattedSize);

public sealed record DirectoryInfo(
    StorageMetadata Metadata,
    string DisplayName,
    DirectorySummary? Summary);

public sealed record StorageEntry(
    string Kind,
    string Path,
    string Name);

internal static class JsonModel
{
    internal static readonly JsonSerializerOptions Options = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
    };
}
