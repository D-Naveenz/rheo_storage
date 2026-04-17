# Rheo.Storage for .NET

`Rheo.Storage` is the `net10.0` managed wrapper over the native `rheo_storage_ffi`
runtime.

It gives .NET applications an object-oriented, path-based API for the Rust core
without pushing .NET object-shape requirements back into the native runtime.

## Supported Platforms

`Rheo.Storage` NuGet packages support:

- Windows `win-x64`
- Windows `win-arm64`

Unsupported platforms are rejected in two places:

- at package-consumption time through a transitive `.targets` file for explicit unsupported RIDs such as `win-x86`
- at runtime through a managed `PlatformNotSupportedException` guard

## Install

```bash
dotnet add package Rheo.Storage
```

## Quick Start

```csharp
using Microsoft.Extensions.Logging;
using Rheo.Storage;

using var loggerFactory = LoggerFactory.Create(builder => builder.AddConsole());
RheoStorage.UseLoggerFactory(loggerFactory);

var file = RheoStorage.File(@"C:\data\sample.pdf");
var info = file.RefreshInformation(includeAnalysis: true);
var bytes = await file.ReadBytesAsync();

var directory = RheoStorage.Directory(@"C:\data");
directory.StartWatching();
directory.Changed += (_, change) => Console.WriteLine(change.Path);
```

## Public API

- `RheoStorage.File(path)` creates a `StorageFile`
- `RheoStorage.Directory(path)` creates a `StorageDirectory`
- `RheoStorage.AnalyzePath(path)` runs point-in-time analysis without creating a wrapper object
- `StorageFile` exposes sync and async methods for analysis, reads, writes, copy, move, rename, and delete
- `StorageDirectory` exposes enumeration, create, copy, move, rename, delete, and explicit watching

## Logging

`Rheo.Storage` integrates with `Microsoft.Extensions.Logging`.

```csharp
using var loggerFactory = LoggerFactory.Create(builder =>
{
    builder.AddConsole();
    builder.SetMinimumLevel(LogLevel.Debug);
});

RheoStorage.UseLoggerFactory(loggerFactory);
```

Once configured, the host receives:

- managed wrapper logs from async handles and orchestration code
- native Rust logs forwarded from `tracing` through the FFI logger bridge

## Packaging Notes

- Local `dotnet build` copies the native DLL into the output folder for development.
- `dotnet pack` is intentionally guarded so local packing cannot silently create a misleading single-runtime package.
- Repository packaging flows stage both native assets before packing:
  - `runtimes/win-x64/native/rheo_storage_ffi.dll`
  - `runtimes/win-arm64/native/rheo_storage_ffi.dll`
