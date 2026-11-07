using System.Collections.Generic;
using System.Linq;

namespace Tct.FrontendFixtures;

public static class Workflow
{
    public static IReadOnlyList<string> CollectCommands(IEnumerable<Step> steps)
    {
        var commands = new List<string>();
        foreach (var step in steps)
        {
            if (!string.IsNullOrWhiteSpace(step.Run))
            {
                commands.Add(step.Run.Trim());
            }
            else if (!string.IsNullOrWhiteSpace(step.Uses))
            {
                commands.Add($"uses:{step.Uses}");
            }
        }

        return commands.Where(command => command.Length > 0).ToArray();
    }
}

public sealed record Step(string Name, string? Run, string? Uses);
