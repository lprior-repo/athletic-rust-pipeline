function determine_scroll (selector, height)
{                
    if ($(selector).length > 0)
    {
        var e = $(selector);
        
        if (e.height() > height)
        {            
            e.css('height', height).css('overflow-y', 'scroll').css('overflow-x', 'hidden').css('padding', '0 5px 0 0');
        }
    }
}    

function allow_stretch (selector, max_height)
{
	if ($(selector).length > 0)
    {
        var e = $(selector).parent();
        
        e.css('height', max_height);
        e.css('overflow', 'hidden');
        
    }	
}