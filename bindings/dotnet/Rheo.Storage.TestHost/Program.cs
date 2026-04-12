using System.Text;
using Rheo.Storage;

var repoRoot = Environment.CurrentDirectory;
var fixturePath = Path.Combine(repoRoot, "rheo_storage", "tests", "fixtures", "sample-2.pdf");
var workDir = Path.Combine(repoRoot, "target", "dotnet-smoke");

if (Directory.Exists(workDir))
{
    Directory.Delete(workDir, recursive: true);
}

Directory.CreateDirectory(workDir);

var analysis = RheoStorage.AnalyzePath(fixturePath);
if (analysis.TopMimeType != "application/pdf")
{
    throw new InvalidOperationException($"Expected application/pdf but got '{analysis.TopMimeType}'.");
}

var fileInfo = RheoStorage.GetFileInfo(fixturePath, includeAnalysis: true);
if (fileInfo.Analysis is null || fileInfo.Analysis.TopMimeType != "application/pdf")
{
    throw new InvalidOperationException("Expected file analysis to be present in file info.");
}

var nestedDir = Path.Combine(workDir, "nested", "inner");
var createdDir = RheoStorage.CreateDirectoryAll(nestedDir);
if (!Directory.Exists(createdDir))
{
    throw new InvalidOperationException("CreateDirectoryAll did not create the expected directory.");
}

var filePath = Path.Combine(nestedDir, "ffi-note.txt");
var writeBytes = Encoding.UTF8.GetBytes("hello from dotnet");
var writtenPath = RheoStorage.WriteFile(filePath, writeBytes);
if (!File.Exists(writtenPath))
{
    throw new InvalidOperationException("WriteFile did not create the expected file.");
}

var readText = RheoStorage.ReadFileText(filePath);
if (readText != "hello from dotnet")
{
    throw new InvalidOperationException("ReadFileText did not round-trip the expected contents.");
}

var copiedPath = Path.Combine(workDir, "copy.txt");
RheoStorage.CopyFile(filePath, copiedPath);
if (!File.Exists(copiedPath))
{
    throw new InvalidOperationException("CopyFile did not create the destination file.");
}

var renamedPath = RheoStorage.RenameFile(copiedPath, "renamed.txt");
if (!File.Exists(renamedPath))
{
    throw new InvalidOperationException("RenameFile did not rename the destination file.");
}

var entries = RheoStorage.ListEntries(workDir, recursive: true);
if (!entries.Any(entry => entry.Name == "ffi-note.txt"))
{
    throw new InvalidOperationException("ListEntries did not include the created file.");
}

var directoryInfo = RheoStorage.GetDirectoryInfo(workDir, includeSummary: true);
if (directoryInfo.Summary is null || directoryInfo.Summary.FileCount == 0)
{
    throw new InvalidOperationException("Directory summary should report at least one file.");
}

RheoStorage.DeleteFile(renamedPath);
RheoStorage.DeleteDirectory(workDir, recursive: true);
if (Directory.Exists(workDir))
{
    throw new InvalidOperationException("DeleteDirectory did not remove the working directory.");
}

Console.WriteLine("Rheo.Storage .NET smoke test completed successfully.");
