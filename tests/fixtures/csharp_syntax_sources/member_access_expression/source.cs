namespace Tct.Contracts;

public sealed class ClockContract
{
    public string Read()
    {
        return DateTime.Now;
    }
}
