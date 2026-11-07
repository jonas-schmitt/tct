namespace Tct.Contracts;

public sealed class Factory
{
    public object Build()
    {
        int[] values = new int[] { 1, 2 };
        int[] empty = new int[0];
        return new Factory();
    }
}
