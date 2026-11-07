#nullable enable

using System;
using System.Collections.Generic;

namespace Tct.FrontendFixtures;

[Serializable]
public sealed class User<TTag>
    where TTag : notnull
{
    public required string Id { get; init; }
    public IReadOnlyList<TTag> Tags { get; init; } = Array.Empty<TTag>();

    public string Describe()
    {
        var tagCount = Tags.Count;
        return $"{Id}:{tagCount}";
    }
}
