int main() 
{
    {
        int x = 10;
        {
            int a = 1;
        }
        x = x + 1;
        return x;
    }
}