def add_console_logs(selection: str) -> str:
    return f"""
<commit_before>
    even_sum = sum([num for num in data if num % 2 == 0])
    odd_sum = sum([num for num in data if num % 2 != 0])
    total_elements = len(data)
    score = (even_sum * odd_sum) / total_elements
    return score
<commit_msg>
added console prints to the code
<commit_after>
    even_sum = sum([num for num in data if num % 2 == 0])
    print(f'even sum: {{even_sum}}')
    odd_sum = sum([num for num in data if num % 2 != 0])
    print(f'odd sum: {{odd_sum}}')
    total_elements = len(data)
    print(f'total elements: {{total_elements}}')
    score = (even_sum * odd_sum) / total_elements
    print(f'score: {{score}}')
    return score
<|endoftext|>

<commit_before>
    def __call__(self, filename: str) -> Optional[str]:
        for filt in self._filters:
            filtered = filt([filename])
            if filtered:
                return filt.language
        return None
<commit_msg>
added console prints to the code
<commit_after>
    def __call__(self, filename: str) -> Optional[str]:
        print(f'filename: {{filename}}')
        print(f'filters: {{self._filters}}')
        for it, filt in enumerate(self._filters):
            print(f'it: {{it}}')
            print(f'filt: {{filt}}')
            filtered = filt([filename])
            print(f'filtered: {{filtered}}')'
            if filtered:
                print(f'language: {{filt.language}}')
                return filt.language
        print(f'language: None')
        return None
<|endoftext|>


<commit_before>
function addNumbers(a, b) {{
    return a + b;
}}
<commit_msg>
added console prints to the code
<commit_after>
function addNumbers(a, b) {{
    console.log('a: ' + a')
    console.log('b: ' + b)
    console.log('a + b: ' + a + b)
    return a + b;
}}
<|endoftext|>


<commit_before>
{selection}
<commit_msg>
added console prints to the code
<commit_after>
"""


def fix_bug(selection: str) -> str:
    return f"""
<commit_before>
    def __call__(self, filename: str) -> Optional[str]:
        for (filt in self._filters):
            filtered = filt([filename)
            if (filtered):
                return filt.language
        return null
<commit_msg>
bugfix
<commit_after>
    def __call__(self, filename: str) -> Optional[str]:
        for filt in self._filters:
            filtered = filt([filename])
            if filtered:
                return filt.language
        return None
<|endoftext|>


<commit_before>
def 0_polynom(x: float = 0, coefficients: List[float]):
    value := 0
    x_power := 1
    for c in coeffs:
        value += x_power * c
        x_power *= x
    return value
<commit_msg>
bugfix
<commit_after>
def polynom_0(coefficients: List[float], x: float = 0):
    value = 0
    x_power = 1
    for c in coefficients:
        value += x_power * c
        x_power *= x
    return value
<|endoftext|>


<commit_before>
{selection}
<commit_msg>
bugfix
<commit_after>
"""


def time_complexity(selection: str) -> str:
    return f"""
<commit_before>
def foo(n, k):
    accum = 0
    for i in range(n):
        for l in range(k):
            accum += i
    return accum
<commit_msg>
replaced code with its time complexity estimation
<commit_after>
# O(n*k). This is because the function has two nested for loops, each of which
# has a time complexity of O(n) and O(k) respectively. Therefore, the total
# time complexity is the product of the two, which is O(n*k).
<|endoftext|>


<commit_before>
def is_palindrome(s):
  for i in range(len(s)//2):
    if s[i]!=s[-1-i]: return False
  return True
<commit_msg>
replaced code with its time complexity estimation
<commit_after>
# O(n/2) where n is the length of the input string 
# The code iterates through half the string and compares characters in constant time.
<|endoftext|>


<commit_before>
{selection}
<commit_msg>
replaced code with its time complexity estimation
<commit_after>
"""


def explain_code_block(selection: str) -> str:
    return f"""
<commit_before>
    def completion(self, final: bool, tokens_batch: Optional[int] = 25) -> Iterator[Dict[str, str]]:
        tokens_batch: int = self.max_tokens if final else tokens_batch

        return self.completion_stream(
            engine=self._engine,
            tokens_batch=tokens_batch,
            prompt=self.prompt,
            replace_modified=self._replace_modified
        )
<commit_msg>
replaced code with detailed and precise description what it does
<commit_after>
# This code defines a function called "completion"
# which takes two arguments: final which is boolean and tokens_batch, which is optional int
# if final = true, streaming is disabled, and vice versa
# tokens batch will be set to self.max_tokens if streaming is disabled
# it returns a function self._completion_stream, which receives engine, tokens_batch, prompt and replace_modified
# this function returns an iterator of Dicts, where keys and values are both strings
<|endoftext|>


<commit_before>
def is_palindrome(s):
  for i in range(len(s)//2):
    if s[i]!=s[-1-i]: return False
  return True
<commit_msg>
replaced code with detailed and precise description what it does
<commit_after>
# iterate over half of the string
# if the first half of the string is not equal to the second half, return False
# if the string is a palindrome, return True
<|endoftext|>



<commit_before>
{selection}
<commit_msg>
replaced code with detailed and precise description what it does
<commit_after>
"""


def make_code_shorter(selection: str) -> str:
    prefix = f"""
<commit_before>
    if threshold is None:
        threshold = random.randint(0, 100)
    big_numbers = []
    for i in range(0, len(my_list)):
        if my_list[i] > threshold:
            big_numbers.append(my_list[i])
    return big_numbers
<commit_msg>
simplified the code
<commit_after>
    threshold = threshold or random.randint(0, 100)
    return [x for x in my_list if x > threshold]
<|endoftext|>


<commit_before>
longest_a_word = None
for word in words:
    if word.startswith('a') and (longest_a_word is None or len(word) > len(longest_a_word)):
        longest_a_word = word
<commit_msg>
simplified the code
<commit_after>
longest_a_word = max((word for word in words if word.startswith('a')), key=len, default=None)
<|endoftext|>


<commit_before>
        self.people: List[Person] = []
        for p in people:
            name = p[0]
            age = p[1]
            person = Person(name, age)
            self.people.append(person)
<commit_msg>
simplified the code
<commit_after>
        self.people: List[Person] = [Person(name, age) for name, age in people]
<|endoftext|>


<commit_before>
def slow_function(n):
    time.sleep(5)
    return n ** 2


def cached_function(n, cache={{}}):
    if n in cache:
        return cache[n]
    else:
        result = slow_function(n)
        cache[n] = result
        return result
<commit_msg>
simplified the code
<commit_after>
@lru_cache(maxsize=None)
def slow_function(n):
    time.sleep(5)
    return n ** 2


def cached_function(n):
    return slow_function(n)
<|endoftext|>


<commit_before>
{selection}
<commit_msg>
simplified the code
<commit_after>  
"""
    return prefix


def comment_each_line(selection: str,) -> str:
    return f"""
<commit_before>
def count_lines_file(file: Path) -> int:
    with file.open('r') as f:
        for i, _ in enumerate(f.readlines()):
            pass
    return i + 1
<commit_msg>
explained each line of code in comments
<commit_after>  
def count_lines_file(file: Path) -> int:
    # open file in read mode
    with file.open('r') as f:
        # enumerate each line in file
        for i, _ in enumerate(f.readlines()):
            # do nothing, just iterate over each line
            pass
    # return the number of lines in the file
    return i + 1
<|endoftext|>


<commit_before>
    TasksSetup(
        status_only=status_only,
        reserve=reserve,
        reset=reset,
        delete=delete,
        verbose=verbose
    )()
<commit_msg>
explained each line of code in comments
<commit_after>  
    TasksSetup(
        # if True, only status of tasks will be printed
        status_only=status_only,
        # if True, tasks will be reserved for current user
        reserve=reserve,
        # if True, tasks will be reset
        reset=reset,
        # if True, tasks will be deleted
        delete=delete,
        # if True, more detailed information will be printed
        verbose=verbose
    )()
<|endoftext|>


<commit_before>
    def _read_save_file(self):
        with self._geo_save_file.open('r') as f:
            for line in f:
                yield json.loads(line)
<commit_msg>
explained each line of code in comments
<commit_after>  
    def _read_save_file(self):
        # open file in read mode
        with self._geo_save_file.open('r') as f:
            # iterate over each line in the file
            for line in f:
                # parse each line as JSON and yield the result
                yield json.loads(line)
<|endoftext|>


<commit_before>
{selection}
<commit_msg>
explained each line of code in comments
<commit_after>
"""
