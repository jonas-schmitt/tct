namespace Tct.Contracts;

public sealed class ChangeNotifier
{
    public string Notify()
    {
        PropertyChanged?.Invoke(this);
        return "sent";
    }
}
