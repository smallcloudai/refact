class Calculator:
    """
    This class represents a simple calculator.

    Attributes:
    name (str): The name of the calculator.
    """

    def __init__(self, name):
        """
        The constructor for the Calculator class.

        Parameters:
        name (str): The name of the calculator.
        """
        self.name = name

    def add(self, x, y):
        """
        Adds two numbers and returns the result.

        Parameters:
        x (int): The first number.
        y (int): The second number.

        Returns:
        int: The sum of x and y.
        """
        return x + y

    def subtract(self, x, y):
        """
        Subtracts one number from another and returns the result.

        Parameters:
        x (int): The number to be subtracted from.
        y (int): The number to subtract.

        Returns:
        int: The result of subtracting y from x.
        """
        return x - y