using System.Runtime.InteropServices;
using Rheo.Storage.Models.Analysis;
using Rheo.Storage.Models.Information;

namespace Rheo.Storage.Interop.Native;

internal static partial class NativeQueries
{
    private const string LibraryName = NativeMemory.LibraryName;

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_analyze_path(string path, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_get_file_info(string path, byte includeAnalysis, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_get_directory_info(string path, byte includeSummary, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_list_files(string path, byte recursive, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_list_directories(string path, byte recursive, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_list_entries(string path, byte recursive, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_read_file(string path, out nint bytesPtr, out nuint bytesLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_read_file_text(string path, out nint stringPtr, out nuint stringLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static unsafe partial NativeStatus rheo_write_file(string path, byte* dataPtr, nuint dataLen, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_write_file_text(string path, string text, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_copy_file(string source, string destination, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_move_file(string source, string destination, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_rename_file(string source, string newName, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_delete_file(string path, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_create_directory(string path, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_create_directory_all(string path, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_copy_directory(string source, string destination, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_move_directory(string source, string destination, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_rename_directory(string source, string newName, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus rheo_delete_directory(string path, byte recursive, out nint errorPtr, out nuint errorLen);
}

internal static class NativeQueryInvoker
{
    internal static AnalysisReport AnalyzePath(string path) =>
        NativeCallInvoker.InvokeJson<NativeAnalysisReportDto>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_analyze_path(path, out dataPtr, out dataLen, out errorPtr, out errorLen))
        .ToModel();

    internal static FileInformation GetFileInformation(string path, bool includeAnalysis) =>
        NativeCallInvoker.InvokeJson<NativeFileInformationDto>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_get_file_info(path, NativeHelpers.ToNativeBool(includeAnalysis), out dataPtr, out dataLen, out errorPtr, out errorLen))
        .ToModel();

    internal static DirectoryInformation GetDirectoryInformation(string path, bool includeSummary) =>
        NativeCallInvoker.InvokeJson<NativeDirectoryInformationDto>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_get_directory_info(path, NativeHelpers.ToNativeBool(includeSummary), out dataPtr, out dataLen, out errorPtr, out errorLen))
        .ToModel();

    internal static IReadOnlyList<StorageEntry> ListFiles(string path, bool recursive) =>
        NativeCallInvoker.InvokeJson<NativeStorageEntryDto[]>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_list_files(path, NativeHelpers.ToNativeBool(recursive), out dataPtr, out dataLen, out errorPtr, out errorLen))
        .Select(static entry => entry.ToModel())
        .ToArray();

    internal static IReadOnlyList<StorageEntry> ListDirectories(string path, bool recursive) =>
        NativeCallInvoker.InvokeJson<NativeStorageEntryDto[]>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_list_directories(path, NativeHelpers.ToNativeBool(recursive), out dataPtr, out dataLen, out errorPtr, out errorLen))
        .Select(static entry => entry.ToModel())
        .ToArray();

    internal static IReadOnlyList<StorageEntry> ListEntries(string path, bool recursive) =>
        NativeCallInvoker.InvokeJson<NativeStorageEntryDto[]>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_list_entries(path, NativeHelpers.ToNativeBool(recursive), out dataPtr, out dataLen, out errorPtr, out errorLen))
        .Select(static entry => entry.ToModel())
        .ToArray();

    internal static byte[] ReadFileBytes(string path) =>
        NativeCallInvoker.InvokeBytes(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_read_file(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string ReadFileText(string path) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_read_file_text(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static unsafe string WriteFileBytes(string path, byte[] content)
    {
        fixed (byte* ptr = content)
        {
            var status = NativeQueries.rheo_write_file(path, ptr, (nuint)content.Length, out var dataPtr, out var dataLen, out var errorPtr, out var errorLen);
            NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
            return NativeMemory.ReadUtf8AndFree(dataPtr, dataLen);
        }
    }

    internal static string WriteFileText(string path, string text) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_write_file_text(path, text, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string CopyFile(string source, string destination) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_copy_file(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string MoveFile(string source, string destination) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_move_file(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string RenameFile(string source, string newName) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_rename_file(source, newName, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static void DeleteFile(string path) =>
        NativeCallInvoker.InvokeUnit((out nint errorPtr, out nuint errorLen) => NativeQueries.rheo_delete_file(path, out errorPtr, out errorLen));

    internal static string CreateDirectory(string path) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_create_directory(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string CreateDirectoryAll(string path) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_create_directory_all(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string CopyDirectory(string source, string destination) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_copy_directory(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string MoveDirectory(string source, string destination) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_move_directory(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string RenameDirectory(string source, string newName) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.rheo_rename_directory(source, newName, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static void DeleteDirectory(string path, bool recursive) =>
        NativeCallInvoker.InvokeUnit((out nint errorPtr, out nuint errorLen) => NativeQueries.rheo_delete_directory(path, NativeHelpers.ToNativeBool(recursive), out errorPtr, out errorLen));
}
