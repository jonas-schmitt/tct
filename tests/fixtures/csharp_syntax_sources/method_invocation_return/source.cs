namespace Tct.Contracts;

public sealed class Greeter
{
    public string Greet()
    {
        return Format();
    }

    public string Format()
    {
        return "world";
    }
}
