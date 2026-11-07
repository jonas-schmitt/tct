namespace Tct.Contracts;

public sealed class ConstructorContract
{
    public string Name { get; }

    public ConstructorContract(string name)
    {
        Name = name;
    }
}
