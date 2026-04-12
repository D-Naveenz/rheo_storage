using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace Rheo.Storage;

public static class RheoStorage
{
    public static AnalysisReport AnalyzePath(string path) =>
        InvokeJson<AnalysisReport>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_analyze_path(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static FileInfo GetFileInfo(string path, bool includeAnalysis = false) =>
        InvokeJson<FileInfo>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_get_file_info(path, ToNativeBool(includeAnalysis), out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static DirectoryInfo GetDirectoryInfo(string path, bool includeSummary = false) =>
        InvokeJson<DirectoryInfo>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_get_directory_info(path, ToNativeBool(includeSummary), out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static IReadOnlyList<StorageEntry> ListFiles(string path, bool recursive = false) =>
        InvokeJson<IReadOnlyList<StorageEntry>>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_list_files(path, ToNativeBool(recursive), out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static IReadOnlyList<StorageEntry> ListDirectories(string path, bool recursive = false) =>
        InvokeJson<IReadOnlyList<StorageEntry>>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_list_directories(path, ToNativeBool(recursive), out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static IReadOnlyList<StorageEntry> ListEntries(string path, bool recursive = false) =>
        InvokeJson<IReadOnlyList<StorageEntry>>(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_list_entries(path, ToNativeBool(recursive), out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static byte[] ReadFileBytes(string path) =>
        InvokeBytes(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_read_file(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static string ReadFileText(string path) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_read_file_text(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static unsafe string WriteFile(string path, byte[] bytes)
    {
        fixed (byte* bytesPtr = bytes)
        {
            var status = NativeMethods.rheo_write_file(
                path,
                bytesPtr,
                (nuint)bytes.Length,
                out var dataPtr,
                out var dataLen,
                out var errorPtr,
                out var errorLen);
            ThrowIfFailed(status, errorPtr, errorLen);
            FreeError(errorPtr, errorLen);
            return PtrToStringAndFree(dataPtr, dataLen);
        }
    }

    public static string WriteFileText(string path, string text) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_write_file_text(path, text, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static string CopyFile(string source, string destination) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_copy_file(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static string MoveFile(string source, string destination) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_move_file(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static string RenameFile(string source, string newName) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_rename_file(source, newName, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static void DeleteFile(string path) =>
        InvokeUnit((out nint errorPtr, out nuint errorLen) => NativeMethods.rheo_delete_file(path, out errorPtr, out errorLen));

    public static string CreateDirectory(string path) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_create_directory(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static string CreateDirectoryAll(string path) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_create_directory_all(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static string CopyDirectory(string source, string destination) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_copy_directory(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static string MoveDirectory(string source, string destination) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_move_directory(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static string RenameDirectory(string source, string newName) =>
        InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeMethods.rheo_rename_directory(source, newName, out dataPtr, out dataLen, out errorPtr, out errorLen));

    public static void DeleteDirectory(string path, bool recursive = true) =>
        InvokeUnit((out nint errorPtr, out nuint errorLen) => NativeMethods.rheo_delete_directory(path, ToNativeBool(recursive), out errorPtr, out errorLen));

    private static byte ToNativeBool(bool value) => value ? (byte)1 : (byte)0;

    private static T InvokeJson<T>(NativeDataCall call)
    {
        var json = InvokeString(call);
        return JsonSerializer.Deserialize<T>(json, JsonModel.Options)
            ?? throw new InvalidOperationException("Native JSON payload could not be deserialized.");
    }

    private static string InvokeString(NativeDataCall call)
    {
        var status = call(out var dataPtr, out var dataLen, out var errorPtr, out var errorLen);
        ThrowIfFailed(status, errorPtr, errorLen);
        FreeError(errorPtr, errorLen);
        return PtrToStringAndFree(dataPtr, dataLen);
    }

    private static byte[] InvokeBytes(NativeDataCall call)
    {
        var status = call(out var dataPtr, out var dataLen, out var errorPtr, out var errorLen);
        ThrowIfFailed(status, errorPtr, errorLen);
        FreeError(errorPtr, errorLen);
        return PtrToBytesAndFree(dataPtr, dataLen);
    }

    private static void InvokeUnit(NativeUnitCall call)
    {
        var status = call(out var errorPtr, out var errorLen);
        ThrowIfFailed(status, errorPtr, errorLen);
        FreeError(errorPtr, errorLen);
    }

    private static void ThrowIfFailed(RheoStatus status, nint errorPtr, nuint errorLen)
    {
        if (status == RheoStatus.Ok)
        {
            return;
        }

        var errorJson = errorPtr == 0 ? null : PtrToStringAndFree(errorPtr, errorLen);
        if (errorJson is null)
        {
            throw new RheoStorageException($"Native call failed with status {status}.", status.ToString());
        }

        var error = JsonSerializer.Deserialize<NativeError>(errorJson, JsonModel.Options)
            ?? throw new RheoStorageException(errorJson, status.ToString());
        throw new RheoStorageException(error.Message, error.Code, error.Path, error.Operation);
    }

    private static void FreeError(nint errorPtr, nuint errorLen)
    {
        if (errorPtr != 0)
        {
            NativeMethods.rheo_string_free(errorPtr, errorLen);
        }
    }

    private static string PtrToStringAndFree(nint ptr, nuint len)
    {
        if (ptr == 0 || len == 0)
        {
            return string.Empty;
        }

        var bytes = new byte[(int)len];
        Marshal.Copy(ptr, bytes, 0, bytes.Length);
        NativeMethods.rheo_string_free(ptr, len);
        return Encoding.UTF8.GetString(bytes);
    }

    private static byte[] PtrToBytesAndFree(nint ptr, nuint len)
    {
        if (ptr == 0 || len == 0)
        {
            return [];
        }

        var bytes = new byte[(int)len];
        Marshal.Copy(ptr, bytes, 0, bytes.Length);
        NativeMethods.rheo_bytes_free(ptr, len);
        return bytes;
    }

    private delegate RheoStatus NativeDataCall(
        out nint dataPtr,
        out nuint dataLen,
        out nint errorPtr,
        out nuint errorLen);

    private delegate RheoStatus NativeUnitCall(out nint errorPtr, out nuint errorLen);
}
