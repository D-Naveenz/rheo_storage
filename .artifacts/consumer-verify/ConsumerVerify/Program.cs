using Rheo.Storage;

var root = Path.Combine(Path.GetTempPath(), "rheo-storage-aot-check", Guid.NewGuid().ToString("N"));
Directory.CreateDirectory(root);

var file = RheoStorage.File(Path.Combine(root, "sample.txt"));
file.WriteText("native aot check");

var text = file.ReadText();
var info = file.RefreshInformation(includeAnalysis: false);

Console.WriteLine($"{info.Name}|{text}|{info.Size}");
