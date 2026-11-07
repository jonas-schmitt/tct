namespace Tct.Contracts;

public sealed class PatternGuard
{
    public bool IsMissing(string? value)
    {
        return value is null;
    }
}
