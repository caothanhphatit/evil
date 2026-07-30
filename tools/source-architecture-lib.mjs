export function countSourceLines(source) {
  return source.length === 0 ? 0 : source.replace(/\r\n/g, "\n").split("\n").length;
}

export function evaluateLineBudget(path, lines, ceiling, target) {
  return {
    path,
    lines,
    ceiling,
    target,
    exceedsCeiling: lines > ceiling,
    exceedsTarget: lines > target,
  };
}

export function forbiddenImports(source, forbiddenSegments) {
  const imports = [...source.matchAll(/(?:from\s+|import\s*\()["']([^"']+)["']/g)]
    .map((match) => match[1]);
  return imports.filter((specifier) => forbiddenSegments.some((segment) => specifier.includes(segment)));
}

export function forbiddenRustDependencies(source, forbiddenModules) {
  return forbiddenModules.filter((moduleName) => (
    source.includes(`crate::${moduleName}`)
    || source.includes(`super::${moduleName}`)
    || source.includes(`super::super::${moduleName}`)
  ));
}
