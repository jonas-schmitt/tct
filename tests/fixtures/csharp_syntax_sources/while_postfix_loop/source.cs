namespace Tct.Contracts;

public sealed class LoopContract
{
    public int Count()
    {
        int index = 0;
        while (index < 3)
        {
            index++;
        }
        return index;
    }
}
