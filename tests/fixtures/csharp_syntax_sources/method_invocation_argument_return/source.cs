namespace Tct.Contracts;

public sealed class Greeter
{
    public string Greet()
    {
        return Format("world");
    }

    public string Format(string value)
    {
        return value;
    }
}
