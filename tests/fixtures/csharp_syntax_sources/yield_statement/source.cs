namespace Tct.Contracts;

public sealed class SequenceFactory
{
    public object Values()
    {
        yield return "world";
    }
}
