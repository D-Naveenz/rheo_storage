namespace Rheo.Storage.Abstractions;

/// <summary>
/// Represents a path-based storage item surfaced by the Rheo Storage .NET bindings.
/// </summary>
public interface IStorageItem : IDisposable
{
    /// <summary>
    /// Gets the absolute file-system path represented by this instance.
    /// </summary>
    string FullPath { get; }

    /// <summary>
    /// Gets the last path segment for the current item.
    /// </summary>
    string Name { get; }

    /// <summary>
    /// Gets a value indicating whether the represented path currently exists.
    /// </summary>
    bool Exists { get; }
}
