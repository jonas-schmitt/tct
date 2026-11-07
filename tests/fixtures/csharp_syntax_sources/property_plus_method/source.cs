namespace Tct.Contracts;

public sealed class Greeter
{
    public string Name { get; } = "world";

    public string Greet()
    {
        return Name;
    }
}
