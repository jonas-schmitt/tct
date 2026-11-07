namespace Tct.Contracts;

public sealed class Selector
{
    public int[] Window(int[] values)
    {
        return values[1..2];
    }
}
