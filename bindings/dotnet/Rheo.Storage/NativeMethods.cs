using System.Runtime.InteropServices;

namespace Rheo.Storage;

internal enum RheoStatus
{
    Ok = 0,
    Error = 1,
    InvalidArgument = 2,
    Panic = 3,
}

internal static partial class NativeMethods
{
    internal const string LibraryName = "rheo_storage_ffi";

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_analyze_path(
        string path,
        out nint jsonPtr,
        out nuint jsonLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_get_file_info(
        string path,
        byte includeAnalysis,
        out nint jsonPtr,
        out nuint jsonLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_get_directory_info(
        string path,
        byte includeSummary,
        out nint jsonPtr,
        out nuint jsonLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_list_files(
        string path,
        byte recursive,
        out nint jsonPtr,
        out nuint jsonLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_list_directories(
        string path,
        byte recursive,
        out nint jsonPtr,
        out nuint jsonLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_list_entries(
        string path,
        byte recursive,
        out nint jsonPtr,
        out nuint jsonLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_read_file(
        string path,
        out nint bytesPtr,
        out nuint bytesLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_read_file_text(
        string path,
        out nint stringPtr,
        out nuint stringLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static unsafe partial RheoStatus rheo_write_file(
        string path,
        byte* dataPtr,
        nuint dataLen,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_write_file_text(
        string path,
        string text,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_copy_file(
        string source,
        string destination,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_move_file(
        string source,
        string destination,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_rename_file(
        string source,
        string newName,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_delete_file(
        string path,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_create_directory(
        string path,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_create_directory_all(
        string path,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_copy_directory(
        string source,
        string destination,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_move_directory(
        string source,
        string destination,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_rename_directory(
        string source,
        string newName,
        out nint pathPtr,
        out nuint pathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial RheoStatus rheo_delete_directory(
        string path,
        byte recursive,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName)]
    internal static partial void rheo_string_free(nint ptr, nuint len);

    [LibraryImport(LibraryName)]
    internal static partial void rheo_bytes_free(nint ptr, nuint len);
}
