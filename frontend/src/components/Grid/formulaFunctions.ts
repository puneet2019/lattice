/** Formula function entry with name and signature. */
export interface FormulaFunction {
  name: string;
  signature: string;
  description: string;
}

/** Spreadsheet formula functions with signatures and descriptions. */
export const FORMULA_FUNCTIONS: FormulaFunction[] = [
  // Math
  { name: 'ABS', signature: 'ABS(value)', description: 'Absolute value' },
  { name: 'ACOS', signature: 'ACOS(value)', description: 'Inverse cosine' },
  { name: 'ASIN', signature: 'ASIN(value)', description: 'Inverse sine' },
  { name: 'ATAN', signature: 'ATAN(value)', description: 'Inverse tangent' },
  { name: 'ATAN2', signature: 'ATAN2(x, y)', description: 'Angle from coordinates' },
  { name: 'AVERAGE', signature: 'AVERAGE(value1, [value2, ...])', description: 'Average of values' },
  { name: 'AVERAGEIF', signature: 'AVERAGEIF(range, criteria, [avg_range])', description: 'Average if condition met' },
  { name: 'CEILING', signature: 'CEILING(value, [factor])', description: 'Round up to nearest factor' },
  { name: 'COS', signature: 'COS(angle)', description: 'Cosine of angle' },
  { name: 'COUNT', signature: 'COUNT(value1, [value2, ...])', description: 'Count numeric values' },
  { name: 'COUNTA', signature: 'COUNTA(value1, [value2, ...])', description: 'Count non-empty values' },
  { name: 'COUNTBLANK', signature: 'COUNTBLANK(range)', description: 'Count empty cells' },
  { name: 'COUNTIF', signature: 'COUNTIF(range, criteria)', description: 'Count cells matching criteria' },
  { name: 'COUNTIFS', signature: 'COUNTIFS(range1, criteria1, [range2, criteria2, ...])', description: 'Count cells matching multiple criteria' },
  { name: 'EXP', signature: 'EXP(exponent)', description: 'e raised to a power' },
  { name: 'FLOOR', signature: 'FLOOR(value, [factor])', description: 'Round down to nearest factor' },
  { name: 'INT', signature: 'INT(value)', description: 'Round down to integer' },
  { name: 'LN', signature: 'LN(value)', description: 'Natural logarithm' },
  { name: 'LOG', signature: 'LOG(value, [base])', description: 'Logarithm' },
  { name: 'LOG10', signature: 'LOG10(value)', description: 'Base-10 logarithm' },
  { name: 'MAX', signature: 'MAX(value1, [value2, ...])', description: 'Maximum value' },
  { name: 'MEDIAN', signature: 'MEDIAN(value1, [value2, ...])', description: 'Median value' },
  { name: 'MIN', signature: 'MIN(value1, [value2, ...])', description: 'Minimum value' },
  { name: 'MOD', signature: 'MOD(dividend, divisor)', description: 'Remainder after division' },
  { name: 'PI', signature: 'PI()', description: 'Value of pi' },
  { name: 'POWER', signature: 'POWER(base, exponent)', description: 'Number raised to power' },
  { name: 'PRODUCT', signature: 'PRODUCT(value1, [value2, ...])', description: 'Product of values' },
  { name: 'RAND', signature: 'RAND()', description: 'Random number 0-1' },
  { name: 'RANDBETWEEN', signature: 'RANDBETWEEN(low, high)', description: 'Random integer in range' },
  { name: 'ROUND', signature: 'ROUND(value, [places])', description: 'Round to places' },
  { name: 'ROUNDDOWN', signature: 'ROUNDDOWN(value, [places])', description: 'Round down to places' },
  { name: 'ROUNDUP', signature: 'ROUNDUP(value, [places])', description: 'Round up to places' },
  { name: 'SIGN', signature: 'SIGN(value)', description: 'Sign of value (-1, 0, or 1)' },
  { name: 'SIN', signature: 'SIN(angle)', description: 'Sine of angle' },
  { name: 'SQRT', signature: 'SQRT(value)', description: 'Square root' },
  { name: 'STDEV', signature: 'STDEV(value1, [value2, ...])', description: 'Standard deviation' },
  { name: 'SUBTOTAL', signature: 'SUBTOTAL(function_num, ref1, [ref2, ...])', description: 'Subtotal with function' },
  { name: 'SUM', signature: 'SUM(value1, [value2, ...])', description: 'Sum of values' },
  { name: 'SUMIF', signature: 'SUMIF(range, criteria, [sum_range])', description: 'Sum if condition met' },
  { name: 'SUMIFS', signature: 'SUMIFS(sum_range, range1, criteria1, [range2, criteria2, ...])', description: 'Sum with multiple criteria' },
  { name: 'SUMPRODUCT', signature: 'SUMPRODUCT(array1, [array2, ...])', description: 'Sum of products' },
  { name: 'TAN', signature: 'TAN(angle)', description: 'Tangent of angle' },
  { name: 'TRUNC', signature: 'TRUNC(value, [places])', description: 'Truncate to integer' },
  { name: 'VAR', signature: 'VAR(value1, [value2, ...])', description: 'Variance' },
  // Text
  { name: 'CHAR', signature: 'CHAR(number)', description: 'Character from code' },
  { name: 'CLEAN', signature: 'CLEAN(text)', description: 'Remove non-printable chars' },
  { name: 'CODE', signature: 'CODE(text)', description: 'Character code' },
  { name: 'CONCAT', signature: 'CONCAT(text1, [text2, ...])', description: 'Join text' },
  { name: 'CONCATENATE', signature: 'CONCATENATE(string1, [string2, ...])', description: 'Join strings' },
  { name: 'EXACT', signature: 'EXACT(text1, text2)', description: 'Case-sensitive comparison' },
  { name: 'FIND', signature: 'FIND(search, text, [start])', description: 'Find text position' },
  { name: 'LEFT', signature: 'LEFT(text, [count])', description: 'Left characters' },
  { name: 'LEN', signature: 'LEN(text)', description: 'Text length' },
  { name: 'LOWER', signature: 'LOWER(text)', description: 'Convert to lowercase' },
  { name: 'MID', signature: 'MID(text, start, count)', description: 'Extract middle characters' },
  { name: 'PROPER', signature: 'PROPER(text)', description: 'Capitalize first letters' },
  { name: 'REPLACE', signature: 'REPLACE(text, start, count, new_text)', description: 'Replace characters' },
  { name: 'REPT', signature: 'REPT(text, times)', description: 'Repeat text' },
  { name: 'RIGHT', signature: 'RIGHT(text, [count])', description: 'Right characters' },
  { name: 'SEARCH', signature: 'SEARCH(search, text, [start])', description: 'Find text (case-insensitive)' },
  { name: 'SUBSTITUTE', signature: 'SUBSTITUTE(text, old, new, [instance])', description: 'Replace text' },
  { name: 'TEXT', signature: 'TEXT(value, format)', description: 'Format number as text' },
  { name: 'TEXTJOIN', signature: 'TEXTJOIN(delimiter, ignore_empty, text1, [text2, ...])', description: 'Join text with delimiter' },
  { name: 'TRIM', signature: 'TRIM(text)', description: 'Remove extra spaces' },
  { name: 'UPPER', signature: 'UPPER(text)', description: 'Convert to uppercase' },
  { name: 'VALUE', signature: 'VALUE(text)', description: 'Convert text to number' },
  // Lookup
  { name: 'HLOOKUP', signature: 'HLOOKUP(key, range, index, [sorted])', description: 'Horizontal lookup' },
  { name: 'INDEX', signature: 'INDEX(range, row, [col])', description: 'Value at position' },
  { name: 'INDIRECT', signature: 'INDIRECT(ref_text)', description: 'Reference from text' },
  { name: 'MATCH', signature: 'MATCH(key, range, [type])', description: 'Position of value' },
  { name: 'OFFSET', signature: 'OFFSET(ref, rows, cols, [height], [width])', description: 'Offset reference' },
  { name: 'VLOOKUP', signature: 'VLOOKUP(key, range, index, [sorted])', description: 'Vertical lookup' },
  { name: 'XLOOKUP', signature: 'XLOOKUP(key, lookup, return, [not_found], [match_mode], [search_mode])', description: 'Advanced lookup' },
  // Logical
  { name: 'AND', signature: 'AND(logical1, [logical2, ...])', description: 'All conditions true' },
  { name: 'FALSE', signature: 'FALSE()', description: 'Logical false' },
  { name: 'IF', signature: 'IF(condition, value_if_true, [value_if_false])', description: 'Conditional value' },
  { name: 'IFERROR', signature: 'IFERROR(value, error_value)', description: 'Value if no error' },
  { name: 'IFNA', signature: 'IFNA(value, na_value)', description: 'Value if not N/A' },
  { name: 'IFS', signature: 'IFS(condition1, value1, [condition2, value2, ...])', description: 'Multiple conditions' },
  { name: 'NOT', signature: 'NOT(logical)', description: 'Negate logical value' },
  { name: 'OR', signature: 'OR(logical1, [logical2, ...])', description: 'Any condition true' },
  { name: 'SWITCH', signature: 'SWITCH(expr, case1, value1, [case2, value2, ...], [default])', description: 'Match and return value' },
  { name: 'TRUE', signature: 'TRUE()', description: 'Logical true' },
  { name: 'XOR', signature: 'XOR(logical1, [logical2, ...])', description: 'Exclusive or' },
  // Info
  { name: 'ISBLANK', signature: 'ISBLANK(value)', description: 'Check if empty' },
  { name: 'ISERROR', signature: 'ISERROR(value)', description: 'Check if error' },
  { name: 'ISLOGICAL', signature: 'ISLOGICAL(value)', description: 'Check if boolean' },
  { name: 'ISNA', signature: 'ISNA(value)', description: 'Check if N/A' },
  { name: 'ISNUMBER', signature: 'ISNUMBER(value)', description: 'Check if number' },
  { name: 'ISTEXT', signature: 'ISTEXT(value)', description: 'Check if text' },
  { name: 'N', signature: 'N(value)', description: 'Convert to number' },
  { name: 'NA', signature: 'NA()', description: 'Return N/A error' },
  { name: 'TYPE', signature: 'TYPE(value)', description: 'Type of value' },
  // Date
  { name: 'DATE', signature: 'DATE(year, month, day)', description: 'Create date' },
  { name: 'DATEVALUE', signature: 'DATEVALUE(date_text)', description: 'Convert text to date' },
  { name: 'DAY', signature: 'DAY(date)', description: 'Day of month' },
  { name: 'DAYS', signature: 'DAYS(end_date, start_date)', description: 'Days between dates' },
  { name: 'HOUR', signature: 'HOUR(time)', description: 'Hour from time' },
  { name: 'MINUTE', signature: 'MINUTE(time)', description: 'Minute from time' },
  { name: 'MONTH', signature: 'MONTH(date)', description: 'Month from date' },
  { name: 'NOW', signature: 'NOW()', description: 'Current date and time' },
  { name: 'SECOND', signature: 'SECOND(time)', description: 'Second from time' },
  { name: 'TIME', signature: 'TIME(hour, minute, second)', description: 'Create time' },
  { name: 'TODAY', signature: 'TODAY()', description: 'Current date' },
  { name: 'WEEKDAY', signature: 'WEEKDAY(date, [type])', description: 'Day of week' },
  { name: 'YEAR', signature: 'YEAR(date)', description: 'Year from date' },
  // Array
  { name: 'ARRAYFORMULA', signature: 'ARRAYFORMULA(formula)', description: 'Array formula' },
  { name: 'TRANSPOSE', signature: 'TRANSPOSE(range)', description: 'Transpose rows/cols' },
  { name: 'UNIQUE', signature: 'UNIQUE(range)', description: 'Unique values' },
  { name: 'SORT', signature: 'SORT(range, [col], [ascending])', description: 'Sort range' },
  { name: 'FILTER', signature: 'FILTER(range, condition, [if_empty])', description: 'Filter range' },
  // Statistical
  { name: 'LARGE', signature: 'LARGE(range, k)', description: 'K-th largest value' },
  { name: 'SMALL', signature: 'SMALL(range, k)', description: 'K-th smallest value' },
  { name: 'PERCENTILE', signature: 'PERCENTILE(range, percentile)', description: 'Value at percentile' },
  { name: 'RANK', signature: 'RANK(value, range, [order])', description: 'Rank of value' },
  { name: 'MODE', signature: 'MODE(value1, [value2, ...])', description: 'Most frequent value' },
  // Data
  { name: 'QUERY', signature: 'QUERY(data, query, [headers])', description: 'SQL-like query' },
];

/** Extract the function name being typed at the cursor position in a formula. */
export function extractCurrentToken(formula: string): string {
  // formula starts with =, strip it
  const text = formula.startsWith('=') ? formula.slice(1) : formula;
  // Find the last token: go backward from end until we hit a non-alphanumeric char
  let i = text.length - 1;
  while (i >= 0 && /[A-Za-z0-9_]/.test(text[i])) {
    i--;
  }
  return text.slice(i + 1).toUpperCase();
}

/** Filter formula functions by a prefix token. */
export function filterFormulaFunctions(token: string): FormulaFunction[] {
  if (!token || token.length === 0) return [];
  const upper = token.toUpperCase();
  return FORMULA_FUNCTIONS.filter((f) => f.name.startsWith(upper) && f.name !== upper)
    .slice(0, 8); // limit to 8 suggestions
}
