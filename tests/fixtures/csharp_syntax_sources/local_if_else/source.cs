namespace Tct.Contracts;

public sealed class Calculator
{
    public int Build()
    {
        int count = 1;
        if (count > 0)
        {
            count = 7;
        }
        else
        {
            count = 0;
        }
        return count;
    }
}
