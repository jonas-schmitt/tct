namespace Tct.Contracts;

public sealed class MessageContract
{
    public string Format(string name)
    {
        return $"Hello, {name}!";
    }
}
