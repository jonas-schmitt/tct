namespace Tct.Contracts;

public sealed class Calculator
{
    public int Build()
    {
        int count = 1;
        switch (count)
        {
            case 1:
                count = 7;
                break;
        }
        return count;
    }
}
