using Rheo.Storage.Tests.TestSupport;

namespace Rheo.Storage.Tests.Analyzing;

public sealed class StorageAnalysisTests
{
    [Fact]
    public void AnalyzePath_ReturnsReport_ForExistingFile()
    {
        using var temp = new TemporaryDirectory();
        var filePath = temp.PathFor("notes.txt");
        File.WriteAllText(filePath, "hello from rheo storage");

        var report = RheoStorage.AnalyzePath(filePath);

        Assert.True(report.BytesScanned > 0);
        Assert.True(report.FileSize > 0);
    }
}
