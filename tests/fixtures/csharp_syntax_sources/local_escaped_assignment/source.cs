namespace Tct.Contracts;

public sealed class Greeter
{
    public string Greet()
    {
        string greeting = "hello";
        greeting = "hello \"updated\"";
        return greeting;
    }
}
