using System.Runtime.InteropServices;
using System.Text;

namespace Rheo.Storage.Interop.Native;

internal static partial class NativeMemory
{
    internal const string LibraryName = "rheo_storage_ffi";

    [LibraryImport(LibraryName)]
    internal static partial void rheo_string_free(nint ptr, nuint len);

    [LibraryImport(LibraryName)]
    internal static partial void rheo_bytes_free(nint ptr, nuint len);

    internal static string ReadUtf8AndFree(nint ptr, nuint len)
    {
        if (ptr == 0 || len == 0)
        {
            return string.Empty;
        }

        var bytes = new byte[(int)len];
        Marshal.Copy(ptr, bytes, 0, bytes.Length);
        rheo_string_free(ptr, len);
        return Encoding.UTF8.GetString(bytes);
    }

    internal static byte[] ReadBytesAndFree(nint ptr, nuint len)
    {
        if (ptr == 0 || len == 0)
        {
            return [];
        }

        var bytes = new byte[(int)len];
        Marshal.Copy(ptr, bytes, 0, bytes.Length);
        rheo_bytes_free(ptr, len);
        return bytes;
    }
}
